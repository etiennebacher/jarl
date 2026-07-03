use air_r_syntax::{RBinaryExpression, RSyntaxKind, RSyntaxNode};
use biome_rowan::{AstNode, SyntaxNodeCast};
use oak_core::syntax_ext::RIdentifierExt;
use oak_semantic::semantic_index::{Definition, DefinitionKind, ScopeId, SemanticIndex};

use jarl_semantic::{
    SemanticInfo, assignment_lhs_is_complex, assignment_rhs_is_function_def,
    lhs_range_for_definition,
};

use crate::checker::Checker;
use crate::diagnostic::{Diagnostic, Fix, ViolationData};

/// Version added: 0.6.0
///
/// ## What it does
///
/// Detects objects that are defined (i.e. assigned a value) but never used.
///
/// ## Why is this bad?
///
/// Unused assignments are usually a sign of dead code or a bug. Removing them
/// reduces noise.
///
/// ## Examples
///
/// ```r
/// x <- 1   # unused
/// print(y)
/// ```
pub fn unused_object(
    expressions: &[RSyntaxNode],
    semantic: &SemanticIndex,
    cross_file_used: &std::collections::HashSet<String>,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    let Some(first) = expressions.first() else {
        return Ok(());
    };
    let root = first.ancestors().last().unwrap_or_else(|| first.clone());
    let info = SemanticInfo::build(&root, expressions, semantic, &checker.file_path);
    let exports = &checker.namespace_exports;

    let mut diagnostics = Vec::new();
    let top_level = ScopeId::from(0);
    for &scope_id in &info.scope_ids() {
        for (def_id, def) in semantic.definitions(scope_id).iter() {
            if !should_lint_definition(&info, def) {
                continue;
            }
            if info.is_definition_used(scope_id, def_id, def) {
                continue;
            }
            // Top-level bindings are visible to other files — package
            // siblings share the namespace, and `source()` injects them into
            // the sourcing file — so an object read from another file (or
            // exported) is still used.
            if scope_id == top_level
                && (is_exported(semantic, exports, scope_id, def)
                    || is_used_cross_file(semantic, cross_file_used, scope_id, def))
            {
                continue;
            }
            diagnostics.push(make_diagnostic(semantic, scope_id, def, info.root()));
        }
    }
    diagnostics.extend(collect_assignment_pipe_diagnostics(
        expressions,
        semantic,
        &info,
        exports,
        cross_file_used,
    ));

    for d in diagnostics {
        checker.report_diagnostic(Some(d));
    }

    Ok(())
}

fn should_lint_definition(info: &SemanticInfo<'_>, def: &Definition) -> bool {
    match def.kind() {
        DefinitionKind::Parameter(_)
        | DefinitionKind::ForVariable(_)
        | DefinitionKind::SuperAssignment(_)
        | DefinitionKind::Import { .. } => return false,
        DefinitionKind::Assignment(ptr) => {
            let bin = ptr.to_node(info.root());
            if assignment_rhs_is_function_def(&bin) {
                return false;
            }
            // Replacement-function or subset assignment LHS (`names(x) <-`,
            // `x[1] <-`, `x$a <-`): the LHS construct reads `x` so the
            // surrounding binding is still considered used.
            if assignment_lhs_is_complex(&bin) {
                return false;
            }
        }
    }

    // `=` inside a formula RHS is named-arg syntax, not assignment.
    if info.is_in_formula(def.range()) {
        return false;
    }

    // An assignment inside an NSE context (`quote(x <- 2)`, …) is quoted code,
    // not a real definition.
    if info.is_in_nse(def.range()) {
        return false;
    }

    true
}

fn is_exported(
    semantic: &SemanticIndex,
    exports: &std::collections::HashSet<String>,
    scope_id: ScopeId,
    def: &Definition,
) -> bool {
    if exports.is_empty() {
        return false;
    }
    let name = semantic.symbols(scope_id).symbol(def.symbol()).name();
    exports.contains(name)
}

/// True when this top-level binding is read from another file — a sibling in
/// the same package, or a file that `source()`s this one. `cross_file_used`
/// is precomputed from oak's cross-file resolution (see
/// [`crate::db::AnalysisDb::cross_file_used_objects`]).
fn is_used_cross_file(
    semantic: &SemanticIndex,
    cross_file_used: &std::collections::HashSet<String>,
    scope_id: ScopeId,
    def: &Definition,
) -> bool {
    if cross_file_used.is_empty() {
        return false;
    }
    let name = semantic.symbols(scope_id).symbol(def.symbol()).name();
    cross_file_used.contains(name)
}

fn make_diagnostic(
    semantic: &SemanticIndex,
    scope_id: ScopeId,
    def: &Definition,
    root: &RSyntaxNode,
) -> Diagnostic {
    let name = semantic
        .symbols(scope_id)
        .symbol(def.symbol())
        .name()
        .to_string();
    let range = lhs_range_for_definition(def, root).unwrap_or_else(|| def.range());
    Diagnostic::new(
        ViolationData::new(
            "unused_object".to_string(),
            format!("Object `{name}` is defined but never used."),
            None,
        ),
        range,
        Fix::empty(),
    )
}

/// Workaround for oak not recognising `%<>%` as an assignment. Walk the
/// AST for `x %<>% f(...)` expressions; if no later use of `x` exists in
/// the same scope (or via closure capture), emit a synthetic
/// `unused_object` diagnostic on the LHS identifier.
fn collect_assignment_pipe_diagnostics(
    expressions: &[RSyntaxNode],
    semantic: &SemanticIndex,
    info: &SemanticInfo<'_>,
    exports: &std::collections::HashSet<String>,
    cross_file_used: &std::collections::HashSet<String>,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for expr in expressions {
        for node in expr.descendants() {
            if node.kind() != RSyntaxKind::R_BINARY_EXPRESSION {
                continue;
            }
            let Some(bin) = node.clone().cast::<RBinaryExpression>() else {
                continue;
            };
            let Ok(op) = bin.operator() else { continue };
            if op.text_trimmed() != "%<>%" {
                continue;
            }
            let Ok(lhs) = bin.left() else { continue };
            if lhs.syntax().kind() != RSyntaxKind::R_IDENTIFIER {
                continue;
            }
            let Some(ident) = lhs.syntax().clone().cast::<air_r_syntax::RIdentifier>() else {
                continue;
            };
            let name = ident.name_text();
            let lhs_range = lhs.syntax().text_trimmed_range();
            let expr_end = bin.syntax().text_trimmed_range().end();

            let (scope_id, _) = semantic.scope_at(lhs_range.start());

            let symbol = semantic.symbols(scope_id).id(&name);
            let later_use = symbol.is_some_and(|sym| {
                semantic
                    .uses(scope_id)
                    .iter()
                    .any(|(_, u)| u.symbol() == sym && u.range().start() >= expr_end)
            });
            let closure_use = info.is_used_in_nested_scope(scope_id, &name);
            let top_level = scope_id == ScopeId::from(0);
            let exported = top_level && exports.contains(&name);
            let cross_file = top_level && cross_file_used.contains(&name);

            if !later_use && !closure_use && !exported && !cross_file {
                out.push(Diagnostic::new(
                    ViolationData::new(
                        "unused_object".to_string(),
                        format!("Object `{name}` is defined but never used."),
                        None,
                    ),
                    lhs_range,
                    Fix::empty(),
                ));
            }
        }
    }
    out
}
