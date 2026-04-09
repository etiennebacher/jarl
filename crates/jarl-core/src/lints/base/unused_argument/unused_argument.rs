use std::collections::HashSet;

use jarl_dfg::*;
use rustc_hash::FxHashSet;

use crate::diagnostic::{Diagnostic, Fix, ViolationData};

/// Check for unused variables
///
/// A variable is "unused" when it is assigned a value but never read
/// afterwards. This covers both top-level scripts and function bodies.
///
/// # Known limitations
///
/// - Variables consumed via meta-programming (`get("x")`, `eval(...)`,
///   `environment()`, `mget(...)`) are not detected as reads and will be
///   flagged as false positives. These should be suppressed via an
///   allowlist or lint suppression comments.
/// - Variables that are the last expression in a function body (implicit
///   return) are considered "used".
///
/// # Examples
///
/// ```r
/// x <- 1
/// y <- 2
/// print(y)
/// # `x` is never used
///
/// # ----------------------------
///
/// x <- 1
/// f <- function(y) {
///   x <- mean(y)
///   x
/// }
/// # The `x` defined in the function is used, but the one defined outside the
/// # function is never used
///
/// # ----------------------------
///
/// x <- 1
/// f <- function() {
///   y <- x + 1
///   y
/// }
/// # `x` is defined in the global environment while used in the function, but
/// # this is a completely valid usage, so nothing is reported.
/// ```
pub fn unused_argument(
    dfg: &DataflowGraph,
    namespace_exports: &HashSet<String>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Collect variable names referenced via string interpolation
    // (e.g. `glue("{x}")`, `cli_alert("{n} items")`).
    // We conservatively treat any `{name}` inside a string literal as a use,
    // regardless of whether the string is passed to an interpolating function.
    let interpolated_names = collect_interpolated_names(&dfg);

    // Collect vertices that are targets of NSE edges (e.g. inside `quote()`).
    // Reads from these vertices should not count as real uses.
    let nse_vertices: FxHashSet<NodeId> = dfg
        .vertices()
        .flat_map(|v| {
            dfg.edges_from(v.id)
                .filter(|(_, bits)| bits.contains(EdgeType::NonStandardEvaluation))
                .map(|(target, _)| target)
        })
        .collect();

    // Detect "self-read" definitions: `x <- f(x)` where the DFG incorrectly
    // resolves the Use of `x` on the RHS to the new Definition being created
    // (instead of the previous one).  When this happens, the previous definition
    // of `x` appears unused even though it was actually consumed.  We collect
    // such variable names so we can suppress false positives on earlier
    // definitions of the same name.
    //
    // Detection: a Use "x" reads Def "x" AND the Use is textually within the
    // assignment expression that creates the Def (i.e. the FunctionCall vertex
    // that Returns the Def).
    let self_read_names: FxHashSet<String> = dfg
        .vertices()
        .filter(|v| v.kind == VertexKind::Definition)
        .filter(|def| {
            // Find the assignment FunctionCall that Returns this definition.
            let assign_range = dfg
                .edges_to(def.id)
                .filter(|(_, bits)| bits.contains(EdgeType::Returns))
                .filter_map(|(from, _)| dfg.vertex(from))
                .filter(|v| v.kind == VertexKind::FunctionCall)
                .map(|v| v.range)
                .next();
            if let Some(range) = assign_range {
                // Check if any Use with the same name reads this Def
                // AND is textually within the assignment's range.
                dfg.edges_to(def.id).any(|(from, bits)| {
                    bits.contains(EdgeType::Reads)
                        && dfg.vertex(from).is_some_and(|v| {
                            v.kind == VertexKind::Use
                                && v.name == def.name
                                && range.contains_range(v.range)
                        })
                })
            } else {
                false
            }
        })
        .map(|def| def.name.clone())
        .collect();

    // Collect all function parameter NodeIds so we can skip them.
    // Unused parameters are a separate lint concern.
    let param_ids: FxHashSet<NodeId> = dfg
        .vertices()
        .filter_map(|v| match &v.data {
            VertexData::FunctionDef { params, .. } => Some(params.iter().copied()),
            _ => None,
        })
        .flatten()
        .collect();

    // Collect all Definition vertices.
    let params: Vec<_> = dfg
        .vertices()
        .filter(|v| v.kind == VertexKind::FunctionParam)
        .collect();

    // dbg!(&namespace_exports);
    dbg!(&dfg);

    for param in &params {
        // Focus on function params only
        if !param_ids.contains(&param.id) {
            continue;
        }

        let parent_function_def = dfg.edges_from(param.id).any(|(target, bits)| {
            bits.contains(EdgeType::DefinedBy)
                && dfg
                    .vertex(target)
                    .is_some_and(|v| v.kind == VertexKind::FunctionDef)
        });

        // Skip names exported by the package's NAMESPACE file.
        if namespace_exports.contains(&param.name) {
            continue;
        }

        // Only flag simple identifier assignments (e.g. `x <- 1`).
        // Skip complex LHS like `attr(x, "foo") <- 1`, `x$y <- 1`,
        // `x[i] <- 1`, and function definitions like `f <- function() ...`.
        if !is_simple_identifier(&param.name) {
            continue;
        }

        // Skip definitions whose value is a function definition.
        let is_function_binding = dfg.edges_from(param.id).any(|(target, bits)| {
            bits.contains(EdgeType::DefinedBy)
                && dfg
                    .vertex(target)
                    .is_some_and(|v| v.kind == VertexKind::FunctionDef)
        });
        if is_function_binding {
            continue;
        }

        // Skip super-assignment definitions (`<<-` / `->>`).
        // These modify a variable in a parent scope as a side-effect
        // and are inherently cross-scope — don't flag them.
        if dfg.is_super_assign(param.id) {
            continue;
        }

        // A definition is "used" if any vertex has a Reads edge pointing to it,
        // excluding reads from vertices inside NSE contexts (e.g. `quote(x)`).
        let is_read = dfg
            .edges_to(param.id)
            .any(|(from, bits)| bits.contains(EdgeType::Reads) && !nse_vertices.contains(&from));

        // Also check if this definition is the value in a DefinedBy edge
        // (i.e. this definition's value flows into another definition —
        // that counts as "used" because it was consumed as an expression).
        let is_consumed = dfg
            .edges_to(param.id)
            .any(|(_, bits)| bits.contains(EdgeType::DefinedBy));

        // Check if this definition is used as an argument to a call.
        let is_arg = dfg
            .edges_to(param.id)
            .any(|(_, bits)| bits.contains(EdgeType::Argument));

        // Check if this definition is returned by a function call or
        // control flow construct.  Exclude Returns edges from assignment
        // operators (`<-`, `<<-`, `->`, `->>`, `=`) since those always
        // exist and don't indicate actual use.
        let is_returned = dfg.edges_to(param.id).any(|(from, bits)| {
            bits.contains(EdgeType::Returns)
                && !dfg.vertex(from).is_some_and(|v| is_assignment_op(&v.name))
        });

        // Check if this variable is referenced via string interpolation.
        let is_interpolated = interpolated_names.contains(param.name.as_str());

        // If a later definition of this same variable has a self-referencing
        // read (e.g. `x <- f(x)`), suppress the diagnostic.  The DFG
        // incorrectly resolves the RHS Use to the new Definition rather than
        // the previous one, making this definition look unused when it is not.
        let suppressed_by_self_read = !is_read
            && !is_consumed
            && !is_arg
            && !is_returned
            && self_read_names.contains(&param.name);

        // If a later definition of the same variable is conditional (has
        // control dependencies from short-circuit operators like `||` / `&&`),
        // the earlier definition may still be needed because the later one
        // might not execute.  E.g. `x <- 1; if (cond || (x <- 2)) print(x)`.
        let shadowed_by_conditional = !is_read
            && !is_consumed
            && !is_arg
            && !is_returned
            && params.iter().any(|other| {
                other.name == param.name
                    && other.id != param.id
                    && other.range.start() > param.range.start()
                    && !other.cds.is_empty()
            });

        // If this definition is inside a loop and a Use of the same variable
        // appears earlier in the loop body, this definition feeds the next
        // iteration and shouldn't be flagged.
        // E.g. `for (...) { f(x); x <- nrow(out) }` — the Use of `x` in
        // `f(x)` on the next iteration reads from this definition.
        let loop_cd = param.cds.iter().find(|cd| cd.by_iteration);
        let feeds_next_iteration = !is_read
            && !is_consumed
            && !is_arg
            && !is_returned
            && loop_cd.is_some_and(|lcd| {
                dfg.vertices().any(|v| {
                    v.kind == VertexKind::Use
                        && v.name == param.name
                        && v.range.start() < param.range.start()
                        && v.cds.iter().any(|cd| cd.by_iteration && cd.id == lcd.id)
                })
            });

        if is_read
            || is_consumed
            || is_arg
            || is_returned
            || is_interpolated
            || suppressed_by_self_read
            || shadowed_by_conditional
            || feeds_next_iteration
        {
            continue;
        }

        let range = param.range;
        let diagnostic = Diagnostic::new(
            ViolationData::new(
                "unused_argument".to_string(),
                format!(
                    "Argument `{}` is defined in the function but never used.",
                    param.name
                ),
                None,
            ),
            range,
            Fix::empty(),
        );

        diagnostics.push(diagnostic);
    }

    diagnostics
}

/// Scan all string-literal Value vertices in the DFG for `{...}`
/// interpolation blocks used by glue, cli, and similar functions.
///
/// Extracts all R identifier tokens found inside `{...}`, including
/// within expressions like `{x + 1}` or `{nrow(df)}`.
fn collect_interpolated_names(dfg: &DataflowGraph) -> FxHashSet<String> {
    let mut names = FxHashSet::default();
    for v in dfg.vertices() {
        if v.kind != VertexKind::Value {
            continue;
        }
        let s = &v.name;
        if !(s.starts_with('"') || s.starts_with('\'')) {
            continue;
        }
        // Find each `{...}` block and extract identifiers from it.
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'{' {
                i += 1;
                let start = i;
                // Find the matching `}`, handling nested braces.
                let mut depth = 1u32;
                while i < bytes.len() && depth > 0 {
                    if bytes[i] == b'{' {
                        depth += 1;
                    } else if bytes[i] == b'}' {
                        depth -= 1;
                    }
                    if depth > 0 {
                        i += 1;
                    }
                }
                let expr = &s[start..i];
                extract_identifiers_from_expr(expr, &mut names);
                if i < bytes.len() {
                    i += 1; // skip closing '}'
                }
            } else {
                i += 1;
            }
        }
    }
    names
}

/// Extract R identifier tokens from an interpolation expression.
///
/// Splits on non-identifier characters and keeps tokens that look like
/// valid R names (start with a letter or `.`, not a keyword).
fn extract_identifiers_from_expr(expr: &str, out: &mut FxHashSet<String>) {
    let r_keywords: &[&str] = &[
        "if",
        "else",
        "for",
        "in",
        "while",
        "repeat",
        "function",
        "return",
        "break",
        "next",
        "TRUE",
        "FALSE",
        "NULL",
        "NA",
        "Inf",
        "NaN",
        "NA_integer_",
        "NA_real_",
        "NA_complex_",
        "NA_character_",
    ];

    let mut start = None;
    for (i, ch) in expr.char_indices() {
        let is_ident_char = ch.is_alphanumeric() || ch == '_' || ch == '.';
        match (is_ident_char, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                let token = &expr[s..i];
                if is_r_identifier(token) && !r_keywords.contains(&token) {
                    out.insert(token.to_string());
                }
                start = None;
            }
            _ => {}
        }
    }
    // Handle token at end of string.
    if let Some(s) = start {
        let token = &expr[s..];
        if is_r_identifier(token) && !r_keywords.contains(&token) {
            out.insert(token.to_string());
        }
    }
}

/// Check if a token looks like a valid R identifier (not a number literal).
fn is_r_identifier(s: &str) -> bool {
    let first = s.chars().next().unwrap_or('0');
    (first.is_alphabetic() || first == '.') && !s.chars().all(|c| c.is_ascii_digit() || c == '.')
}

/// A simple identifier is a bare name like `x` or `my_var`.
/// Complex LHS expressions like `attr(x, "foo")`, `x$y`, `x[i]`
/// are not simple identifiers.
fn is_simple_identifier(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('(')
        && !name.contains('[')
        && !name.contains('$')
        && !name.contains('@')
        && !name.contains(' ')
}

/// Check if a vertex name is an assignment operator.
fn is_assignment_op(name: &str) -> bool {
    matches!(name, "<-" | "<<-" | "->" | "->>" | "=" | "%<>%")
}
