use std::collections::HashSet;

use jarl_dfg::*;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::diagnostic::{Diagnostic, Fix, ViolationData};
use crate::namespace::S3Info;

/// Special functions whose parameters are required by R itself and should
/// never be flagged as unused.
const SPECIAL_FUNCTIONS: &[&str] = &[
    ".onLoad",
    ".onUnload",
    ".onAttach",
    ".onDetach",
    ".Last.lib",
];

/// Check for unused function parameters.
///
/// A function parameter is "unused" when it appears in the function
/// signature but is never referenced in the function body.
///
/// Parameters are skipped (not flagged) when:
/// - The function is an S3 generic (body calls `UseMethod()`)
/// - The function is a registered S3 method (from NAMESPACE)
/// - The function body calls `match.call()` (captures all args)
/// - The function is a special R hook (`.onLoad`, etc.)
/// - The parameter is `...`
///
/// # Examples
///
/// ```r
/// f <- function(x, y) {
///   x + 1
/// }
/// # `y` is never used
/// ```
pub fn unused_argument(
    dfg: &DataflowGraph,
    namespace_exports: &HashSet<String>,
    s3_info: &S3Info,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Collect variable names referenced via string interpolation
    // (e.g. `glue("{x}")`, `cli_alert("{n} items")`).
    let interpolated_names = collect_interpolated_names(dfg);

    // Build a mapping: FunctionDef NodeId → assigned function name.
    // From `f <- function(x) { ... }`, the Definition vertex `f` has a
    // DefinedBy edge to the FunctionDef vertex.
    let mut fdef_to_name: FxHashMap<NodeId, String> = FxHashMap::default();
    for v in dfg.vertices() {
        if v.kind == VertexKind::Definition {
            for (target, bits) in dfg.edges_from(v.id) {
                if bits.contains(EdgeType::DefinedBy)
                    && let Some(tv) = dfg.vertex(target)
                    && tv.kind == VertexKind::FunctionDef
                {
                    fdef_to_name.insert(target, v.name.clone());
                }
            }
        }
    }

    // Identify FunctionDef vertices whose body contains UseMethod(),
    // NextMethod(), standardGeneric(), or match.call() calls.
    //
    // Collect dispatch calls once, then check each FunctionDef against
    // them to avoid O(V²) nested iteration.
    let mut skip_all_params: FxHashSet<NodeId> = FxHashSet::default();
    let dispatch_call_ranges: Vec<_> = dfg
        .vertices()
        .filter(|v| {
            v.kind == VertexKind::FunctionCall
                && matches!(
                    v.name.as_str(),
                    "UseMethod" | "NextMethod" | "standardGeneric" | "match.call"
                )
        })
        .map(|v| v.range)
        .collect();
    if !dispatch_call_ranges.is_empty() {
        for v in dfg.vertices() {
            if v.kind == VertexKind::FunctionDef
                && dispatch_call_ranges
                    .iter()
                    .any(|r| v.range.contains_range(*r))
            {
                skip_all_params.insert(v.id);
            }
        }
    }

    // Skip FunctionDefs passed as condition handler arguments to tryCatch(),
    // try_fetch(), or withCallingHandlers(). These handlers must accept a
    // condition object parameter even if they don't use it.
    //
    // Also skip FunctionDefs passed as the `def` argument to setMethod() or
    // setReplaceMethod(). S4 methods must match the generic's signature, so
    // unused params are expected.
    for v in dfg.vertices() {
        if v.kind != VertexKind::FunctionCall {
            continue;
        }
        if let VertexData::Call { args } = &v.data {
            match v.name.as_str() {
                "tryCatch" | "try_fetch" | "rlang::try_fetch" | "withCallingHandlers" => {
                    for arg in args {
                        if arg.name.is_some()
                            && !matches!(arg.name.as_deref(), Some("expr") | Some("finally"))
                            && dfg
                                .vertex(arg.node_id)
                                .is_some_and(|t| t.kind == VertexKind::FunctionDef)
                        {
                            skip_all_params.insert(arg.node_id);
                        }
                    }
                }
                "setMethod" | "setReplaceMethod" => {
                    // def is the 3rd positional arg or `def = ...`
                    let fdef_id = args
                        .iter()
                        .find(|a| a.name.as_deref() == Some("def"))
                        .or_else(|| args.iter().filter(|a| a.name.is_none()).nth(2))
                        .map(|a| a.node_id);
                    if let Some(id) = fdef_id
                        && dfg
                            .vertex(id)
                            .is_some_and(|t| t.kind == VertexKind::FunctionDef)
                    {
                        skip_all_params.insert(id);
                    }
                }
                _ => {}
            }
        }
    }

    // Also skip FunctionDefs whose assigned name is an S3 generic, S3 method,
    // or a special function.
    for (fdef_id, name) in &fdef_to_name {
        if s3_info.generics.contains(name)
            || s3_info.methods.contains(name)
            || SPECIAL_FUNCTIONS.contains(&name.as_str())
        {
            skip_all_params.insert(*fdef_id);
        }
    }

    // Collect all function parameter NodeIds, grouped by their parent
    // FunctionDef.
    let param_ids: FxHashSet<NodeId> = dfg
        .vertices()
        .filter_map(|v| match &v.data {
            VertexData::FunctionDef { params, .. } => Some(params.iter().copied()),
            _ => None,
        })
        .flatten()
        .collect();

    // Map each param NodeId to its parent FunctionDef NodeId.
    let mut param_to_fdef: FxHashMap<NodeId, NodeId> = FxHashMap::default();
    for v in dfg.vertices() {
        if let VertexData::FunctionDef { params, .. } = &v.data {
            for &pid in params {
                param_to_fdef.insert(pid, v.id);
            }
        }
    }

    // Detect "self-read" definitions: `.cols <- enquo(.cols)` where the DFG
    // resolves the Use of `.cols` on the RHS to the new Definition (instead
    // of the parameter). When this happens, the parameter appears unused
    // even though it was consumed. We collect such variable names so we can
    // suppress false positives on parameters with the same name.
    let self_read_names: FxHashSet<String> = dfg
        .vertices()
        .filter(|v| v.kind == VertexKind::Definition)
        .filter(|def| {
            let assign_range = dfg
                .edges_to(def.id)
                .filter(|(_, bits)| bits.contains(EdgeType::Returns))
                .filter_map(|(from, _)| dfg.vertex(from))
                .filter(|v| v.kind == VertexKind::FunctionCall)
                .map(|v| v.range)
                .next();
            if let Some(range) = assign_range {
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

    let params: Vec<_> = dfg
        .vertices()
        .filter(|v| v.kind == VertexKind::FunctionParam)
        .collect();

    for param in &params {
        // Only consider params that are in a function's param list.
        if !param_ids.contains(&param.id) {
            continue;
        }

        // Skip `...` params.
        if param.name == "..." {
            continue;
        }

        // Skip params of functions we decided to skip entirely.
        if let Some(&fdef_id) = param_to_fdef.get(&param.id)
            && skip_all_params.contains(&fdef_id)
        {
            continue;
        }

        // Skip names exported by the package's NAMESPACE file.
        if namespace_exports.contains(&param.name) {
            continue;
        }

        // A param is "used" if any vertex has a Reads edge pointing to it.
        // Reads from NSE contexts (e.g. `substitute(x)`) still count —
        // the variable is referenced even if not evaluated.
        let is_read = dfg
            .edges_to(param.id)
            .any(|(_, bits)| bits.contains(EdgeType::Reads));

        // Check if the param is used as an argument to a call.
        let is_arg = dfg
            .edges_to(param.id)
            .any(|(_, bits)| bits.contains(EdgeType::Argument));

        // Check if the param is consumed by another definition.
        let is_consumed = dfg
            .edges_to(param.id)
            .any(|(_, bits)| bits.contains(EdgeType::DefinedBy));

        // Check if the param is returned (excluding assignment operators).
        let is_returned = dfg.edges_to(param.id).any(|(from, bits)| {
            bits.contains(EdgeType::Returns)
                && !dfg.vertex(from).is_some_and(|v| is_assignment_op(&v.name))
        });

        // Check if this parameter is referenced via string interpolation.
        let is_interpolated = interpolated_names.contains(param.name.as_str());

        // Suppress false positive from the self-read pattern: the param
        // appears unused because the DFG resolved the RHS use to the new
        // definition instead of the parameter.
        let suppressed_by_self_read = !is_read
            && !is_consumed
            && !is_arg
            && !is_returned
            && self_read_names.contains(&param.name);

        if is_read
            || is_arg
            || is_consumed
            || is_returned
            || is_interpolated
            || suppressed_by_self_read
        {
            continue;
        }

        let diagnostic = Diagnostic::new(
            ViolationData::new(
                "unused_argument".to_string(),
                format!(
                    "Argument `{}` is defined in the function but never used.",
                    param.name
                ),
                None,
            ),
            param.range,
            Fix::empty(),
        );

        diagnostics.push(diagnostic);
    }

    diagnostics
}

/// Check if a vertex name is an assignment operator.
fn is_assignment_op(name: &str) -> bool {
    matches!(name, "<-" | "<<-" | "->" | "->>" | "=" | "%<>%")
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
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'{' {
                i += 1;
                let start = i;
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
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
    }
    names
}

/// Extract R identifier tokens from an interpolation expression.
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
