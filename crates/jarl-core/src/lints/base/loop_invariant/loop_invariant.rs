use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{StatementsExt, get_function_name, get_function_namespace_prefix};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList, Direction, TextRange};
use jarl_semantic::{SemanticInfo, assignment_lhs_name};
use oak_core::syntax_ext::RIdentifierExt;
use oak_semantic::semantic_index::SemanticIndex;
use oak_semantic::{DefinitionId, ScopeId};
use std::collections::HashSet;

pub struct LoopInvariant {
    name: String,
}

/// <!-- docs: start -->
/// Version added: 0.8.0
///
/// ## What it does
///
/// Reports assignments in the body of a `for` loop whose value is the same at
/// every iteration, because it doesn't depend on the loop variable or on any
/// object modified in the loop.
///
/// This rule is conservative: it only reports top-level assignments whose
/// value is built from a known set of base R functions that have no side
/// effects, and it skips loops that contain calls able to modify objects in
/// ways the analysis can't follow (such as `assign()`, `eval()`, or `<<-`).
///
/// ## Why is this bad?
///
/// The value is recomputed at every iteration even though it never changes.
/// Computing it once before the loop is faster and makes clearer what actually
/// depends on the loop.
///
/// Note that moving the assignment before the loop also runs it when the loop
/// has zero iterations.
///
/// ## Example
///
/// ```r
/// for (i in seq_along(files)) {
///   n <- nrow(data)
///   out[[i]] <- process(files[i], n)
/// }
/// ```
///
/// Use instead:
///
/// ```r
/// n <- nrow(data)
/// for (i in seq_along(files)) {
///   out[[i]] <- process(files[i], n)
/// }
/// ```
/// <!-- docs: end -->
impl Violation for LoopInvariant {
    fn rule(&self) -> Rule {
        Rule::LoopInvariant
    }

    fn body(&self) -> String {
        format!(
            "`{}` is computed identically at every iteration of the loop.",
            self.name
        )
    }

    fn suggestion(&self) -> Option<String> {
        Some("Move this assignment before the loop.".to_string())
    }
}

/// Calls that can modify objects in ways the use-def information doesn't
/// record. A loop containing any of them is skipped.
const STATEFUL_FUNCTIONS: &[&str] = &[
    "assign",
    "delayedAssign",
    "makeActiveBinding",
    "rm",
    "remove",
    "source",
    "sys.source",
    "load",
    "attach",
    "detach",
    "library",
    "require",
    "set.seed",
    "RNGkind",
    "options",
    "Sys.setenv",
    "Sys.setlocale",
    "setwd",
    "eval",
    "evalq",
    "local",
    "within",
    "list2env",
    "environment",
    "sys.function",
    "parent.frame",
    "lockBinding",
    "unlockBinding",
    // data.table modifies by reference.
    "set",
    "setattr",
    "setnames",
    "setkey",
    "setkeyv",
    "setorder",
    "setorderv",
    "setDT",
    "setDF",
    "setcolorder",
    "setindex",
    "setindexv",
    "setalloccol",
    // rlang and withr
    "env_bind",
    "env_poke",
    "env_unbind",
    "local_options",
    "local_envvar",
    "local_seed",
    "with_seed",
    "local_dir",
];

/// Base R functions whose result only depends on their arguments.
const PURE_FUNCTIONS: &[&str] = &[
    "abs",
    "sqrt",
    "exp",
    "log",
    "log2",
    "log10",
    "log1p",
    "expm1",
    "floor",
    "ceiling",
    "round",
    "signif",
    "trunc",
    "sin",
    "cos",
    "tan",
    "max",
    "min",
    "pmax",
    "pmin",
    "sum",
    "prod",
    "mean",
    "median",
    "range",
    "var",
    "sd",
    "quantile",
    "cumsum",
    "cumprod",
    "diff",
    "rowSums",
    "colSums",
    "rowMeans",
    "colMeans",
    "crossprod",
    "solve",
    "diag",
    "t",
    "length",
    "lengths",
    "nrow",
    "ncol",
    "NROW",
    "NCOL",
    "dim",
    "names",
    "colnames",
    "rownames",
    "dimnames",
    "levels",
    "nlevels",
    "nchar",
    "seq",
    "seq_len",
    "seq_along",
    "rev",
    "sort",
    "order",
    "unique",
    "duplicated",
    "rep",
    "rep_len",
    "c",
    "list",
    "vector",
    "numeric",
    "character",
    "logical",
    "integer",
    "matrix",
    "array",
    "cbind",
    "rbind",
    "unlist",
    "setdiff",
    "union",
    "intersect",
    "setNames",
    "which",
    "which.max",
    "which.min",
    "match",
    "is.na",
    "is.null",
    "anyNA",
    "any",
    "all",
    "ifelse",
    "isTRUE",
    "isFALSE",
    "identical",
    "inherits",
    "class",
    "paste",
    "paste0",
    "sprintf",
    "toupper",
    "tolower",
    "trimws",
    "substr",
    "substring",
    "strsplit",
    "sub",
    "gsub",
    "grepl",
    "grep",
    "regexpr",
    "gregexpr",
    "startsWith",
    "endsWith",
    "format",
    "formatC",
    "as.numeric",
    "as.double",
    "as.integer",
    "as.character",
    "as.logical",
    "as.vector",
    "as.list",
    "as.matrix",
    "as.factor",
    "factor",
    "file.path",
    "basename",
    "dirname",
    "tabulate",
    "table",
    "head",
    "tail",
];

const PURE_OPERATORS: &[&str] = &["%in%", "%*%", "%o%", "%x%", "%%", "%/%"];

/// A top-level `name <- value` in the loop body whose value has no side effects.
struct Candidate {
    name: String,
    range: TextRange,
    value: TextRange,
    def_id: DefinitionId,
}

pub fn loop_invariant(
    for_statement: &RForStatement,
    semantic: &SemanticIndex,
    info: &SemanticInfo<'_>,
) -> anyhow::Result<Vec<Diagnostic>> {
    let loop_range = for_statement.syntax().text_trimmed_range();
    let Ok(body) = for_statement.body() else {
        return Ok(vec![]);
    };
    if info.is_in_nse(loop_range) {
        return Ok(vec![]);
    }
    let Some(mutated) = mutated_names(&body) else {
        return Ok(vec![]);
    };

    let (scope, _) = semantic.scope_at(body.syntax().text_trimmed_range().start());
    let statements: Vec<AnyRExpression> = match &body {
        AnyRExpression::RBracedStatements(braced) => braced.expressions(),
        _ => vec![body.clone()],
    };
    let candidates: Vec<Candidate> = statements
        .iter()
        .filter_map(|statement| candidate(statement, scope, loop_range, semantic, &mutated))
        .collect();

    // A candidate may read another one defined above it, so iterate until no
    // new invariant is found.
    let mut invariant = vec![false; candidates.len()];
    let mut changed = true;
    while changed {
        changed = false;
        for k in 0..candidates.len() {
            if !invariant[k]
                && reads_are_invariant(
                    &candidates[k],
                    &candidates,
                    &invariant,
                    scope,
                    loop_range,
                    semantic,
                    &mutated,
                )
            {
                invariant[k] = true;
                changed = true;
            }
        }
    }

    Ok(candidates
        .into_iter()
        .zip(invariant)
        .filter(|(_, invariant)| *invariant)
        .map(|(candidate, _)| {
            Diagnostic::new(
                LoopInvariant { name: candidate.name },
                candidate.range,
                Fix::empty(),
            )
        })
        .collect())
}

/// Names modified in place in the loop body (`x[i] <- `, `names(x) <- `,
/// `x$method()`), or `None` if the body can modify objects in ways that can't
/// be tracked.
fn mutated_names(body: &AnyRExpression) -> Option<HashSet<String>> {
    if body
        .syntax()
        .descendants_tokens(Direction::Next)
        .any(|token| {
            matches!(
                token.kind(),
                RSyntaxKind::SUPER_ASSIGN | RSyntaxKind::SUPER_ASSIGN_RIGHT | RSyntaxKind::WALRUS
            )
        })
    {
        return None;
    }

    let mut mutated = HashSet::new();
    for node in body.syntax().descendants() {
        if let Some(call) = RCall::cast(node.clone()) {
            let Ok(function) = call.function() else {
                continue;
            };
            if STATEFUL_FUNCTIONS.contains(&get_function_name(function.clone()).as_str()) {
                return None;
            }
            // `obj$method()` can modify `obj` by reference (R6, environments).
            if let AnyRExpression::RExtractExpression(extract) = function
                && let Some(root) = extract.left().ok().and_then(target_root)
            {
                mutated.insert(root);
            }
        } else if let Some(binary) = RBinaryExpression::cast(node)
            && let Ok(operator) = binary.operator()
        {
            let target = match operator.kind() {
                RSyntaxKind::ASSIGN | RSyntaxKind::EQUAL => binary.left(),
                RSyntaxKind::ASSIGN_RIGHT => binary.right(),
                _ => continue,
            };
            if let Ok(target) = target
                && !matches!(target, AnyRExpression::RIdentifier(_))
                && let Some(root) = target_root(target)
            {
                mutated.insert(root);
            }
        }
    }
    Some(mutated)
}

/// The object modified by a complex assignment target: `x` in `x[i]`,
/// `x$a$b`, `names(x)`.
fn target_root(target: AnyRExpression) -> Option<String> {
    match target {
        AnyRExpression::RIdentifier(identifier) => Some(identifier.name_text()),
        AnyRExpression::RSubset(subset) => target_root(subset.function().ok()?),
        AnyRExpression::RSubset2(subset) => target_root(subset.function().ok()?),
        AnyRExpression::RExtractExpression(extract) => target_root(extract.left().ok()?),
        AnyRExpression::RCall(call) => {
            let first = call.arguments().ok()?.items().iter().next()?.ok()?;
            target_root(first.value()?)
        }
        _ => None,
    }
}

fn candidate(
    statement: &AnyRExpression,
    scope: ScopeId,
    loop_range: TextRange,
    semantic: &SemanticIndex,
    mutated: &HashSet<String>,
) -> Option<Candidate> {
    let binary = statement.as_r_binary_expression()?;
    let name = assignment_lhs_name(binary.syntax())?;
    let value = match binary.operator().ok()?.kind() {
        RSyntaxKind::ASSIGN | RSyntaxKind::EQUAL => binary.right().ok()?,
        RSyntaxKind::ASSIGN_RIGHT => binary.left().ok()?,
        _ => return None,
    };
    if mutated.contains(&name) {
        return None;
    }
    let callees = pure_callees(&value, semantic)?;
    let range = binary.syntax().text_trimmed_range();
    let value_range = value.syntax().text_trimmed_range();

    // Skip values that don't read any object, they are cheap to recompute.
    let reads_object = semantic.uses(scope).iter().any(|(_, use_site)| {
        value_range.contains_range(use_site.range()) && !callees.contains(&use_site.range())
    });
    if !reads_object {
        return None;
    }

    // Hoisting is only correct if this is the only definition of `name` in the
    // loop and nothing in the loop reads `name` before it.
    let symbol = semantic.symbols(scope).id(&name)?;
    let mut definitions = semantic
        .definitions(scope)
        .iter()
        .filter(|(_, def)| def.symbol() == symbol && loop_range.contains_range(def.range()));
    let (def_id, _) = definitions.next()?;
    if definitions.next().is_some() {
        return None;
    }
    if semantic.uses_of(&name).iter().any(|(_, _, use_site)| {
        loop_range.contains_range(use_site.range()) && use_site.range().start() < range.end()
    }) {
        return None;
    }

    Some(Candidate { name, range, value: value_range, def_id })
}

/// The callee ranges of `value` if it has at least one call and every call and
/// operator in it is free of side effects.
fn pure_callees(value: &AnyRExpression, semantic: &SemanticIndex) -> Option<Vec<TextRange>> {
    for token in value.syntax().descendants_tokens(Direction::Next) {
        match token.kind() {
            RSyntaxKind::ASSIGN
            | RSyntaxKind::ASSIGN_RIGHT
            | RSyntaxKind::SUPER_ASSIGN
            | RSyntaxKind::SUPER_ASSIGN_RIGHT
            | RSyntaxKind::WALRUS
            | RSyntaxKind::TILDE => return None,
            RSyntaxKind::SPECIAL if !PURE_OPERATORS.contains(&token.text_trimmed()) => {
                return None;
            }
            _ => {}
        }
    }

    let mut callees = Vec::new();
    for node in value.syntax().descendants() {
        match node.kind() {
            RSyntaxKind::R_FUNCTION_DEFINITION => return None,
            // `(x = 1)` is an assignment.
            RSyntaxKind::R_BINARY_EXPRESSION
                if RBinaryExpression::cast(node.clone())?
                    .operator()
                    .ok()?
                    .kind()
                    == RSyntaxKind::EQUAL =>
            {
                return None;
            }
            RSyntaxKind::R_CALL => {
                let function = RCall::cast(node)?.function().ok()?;
                if !is_pure_function(&function, semantic) {
                    return None;
                }
                callees.push(function.syntax().text_trimmed_range());
            }
            _ => {}
        }
    }
    (!callees.is_empty()).then_some(callees)
}

fn is_pure_function(function: &AnyRExpression, semantic: &SemanticIndex) -> bool {
    let from_base = match get_function_namespace_prefix(function.clone()).as_deref() {
        Some(namespace) => matches!(namespace, "base::" | "stats::" | "utils::"),
        // Not redefined in this file.
        None => {
            function.as_r_identifier().is_some()
                && semantic
                    .use_at(function.syntax().text_trimmed_range().start())
                    .is_some_and(|(scope, use_id, _)| {
                        semantic
                            .reaching_definitions(scope, use_id)
                            .next()
                            .is_none()
                    })
        }
    };
    from_base && PURE_FUNCTIONS.contains(&get_function_name(function.clone()).as_str())
}

/// Whether every object read by `candidate` is defined outside the loop, or
/// by an invariant candidate above it.
fn reads_are_invariant(
    candidate: &Candidate,
    candidates: &[Candidate],
    invariant: &[bool],
    scope: ScopeId,
    loop_range: TextRange,
    semantic: &SemanticIndex,
    mutated: &HashSet<String>,
) -> bool {
    semantic
        .uses(scope)
        .iter()
        .filter(|(_, use_site)| candidate.value.contains_range(use_site.range()))
        .all(|(use_id, use_site)| {
            let name = semantic.symbols(scope).symbol(use_site.symbol()).name();
            !mutated.contains(name)
                && semantic
                    .reaching_definitions(scope, use_id)
                    .all(|(def_scope, def_id)| {
                        !loop_range.contains_range(semantic.definitions(def_scope)[def_id].range())
                            || candidates
                                .iter()
                                .zip(invariant)
                                .any(|(other, is_invariant)| {
                                    *is_invariant
                                        && def_scope == scope
                                        && other.def_id == def_id
                                        && other.range.end() <= use_site.range().start()
                                })
                    })
        })
}
