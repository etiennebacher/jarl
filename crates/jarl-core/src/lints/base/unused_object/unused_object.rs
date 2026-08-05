use air_r_syntax::RSyntaxNode;
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
            // the sourcing file — so an exported object is still used.
            if scope_id == top_level && is_exported(semantic, exports, scope_id, def) {
                continue;
            }
            diagnostics.push(make_diagnostic(semantic, scope_id, def, info.root()));
        }
    }
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
        // A call-created binding (`assign("x", 1)`, `x %<>% f()`). A call that
        // redirects its binding to another environment never reaches here:
        // oak drops the effect when a target-environment argument is supplied
        // (`assign`'s `pos`/`envir`, `delayedAssign`'s `assign.env`), so the
        // name is not recorded as bound in this scope at all.
        DefinitionKind::Assign { .. } => {}
    }

    // `=` inside a formula RHS is named-arg syntax, not assignment.
    if info.is_in_formula(def.range()) {
        return false;
    }

    // An assignment inside an NSE context (`substitute(x <- 2)`, …) is quoted
    // code, not a real definition.
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
