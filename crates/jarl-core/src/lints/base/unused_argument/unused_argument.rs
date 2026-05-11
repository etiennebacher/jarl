use std::collections::HashSet;

use air_r_syntax::{AnyRExpression, RFunctionDefinition, RSyntaxKind, RSyntaxNode};
use biome_rowan::{SyntaxNodeCast, TextRange};
use oak_core::syntax_ext::RIdentifierExt;
use oak_index::semantic_index::{DefinitionKind, ScopeId, SemanticIndex};

use jarl_semantic::SemanticInfo;

use crate::checker::Checker;
use crate::diagnostic::{Diagnostic, Fix, ViolationData};

/// Version added: 0.6.0
///
/// ## What it does
///
/// Detects function parameters that are never read inside the function body.
///
/// ## Why is this bad?
///
/// Unused parameters are usually a sign of dead code or a forgotten rename.
/// They make the function signature misleading.
///
/// ## Examples
///
/// ```r
/// f <- function(x, y) x + 1
/// ```
///
/// `y` is unused — either remove it or use it.
///
/// ## Notes
///
/// Skipped to avoid false positives:
/// - The dots parameter `...`.
/// - Parameters of S3 methods registered in NAMESPACE (`S3method(generic,
///   class)`). The signature is fixed by the generic, so unused params are
///   often unavoidable.
/// - S3/S4 generic functions — those whose body dispatches via `UseMethod()`
///   or `standardGeneric()`. Their parameters are forwarded to the dispatched
///   method, not read locally.
/// - R lifecycle hooks (`.onLoad`, `.onAttach`, `.onDetach`, `.onUnload`,
///   `.Last.lib`, `.First.lib`, `on_load`). These have fixed signatures
///   imposed by the runtime.
pub fn unused_argument(
    expressions: &[RSyntaxNode],
    semantic: &SemanticIndex,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    let info = SemanticInfo::build(expressions, semantic);
    let exports = &checker.namespace_exports;

    let mut diagnostics = Vec::new();
    for &scope_id in &info.scope_ids() {
        if should_skip_function(&info, scope_id, exports) {
            continue;
        }

        for (def_id, def) in semantic.definitions(scope_id).iter() {
            let DefinitionKind::Parameter(node) = def.kind() else {
                continue;
            };
            let name = semantic.symbols(scope_id).symbol_id(def.symbol()).name();

            if is_skipped_parameter_name(name) {
                continue;
            }
            if info.is_definition_used(scope_id, def_id, def) {
                continue;
            }

            let range = parameter_name_range(node).unwrap_or_else(|| node.text_trimmed_range());
            diagnostics.push(make_diagnostic(name, range));
        }
    }

    for d in diagnostics {
        checker.report_diagnostic(Some(d));
    }

    Ok(())
}

/// Skip the whole function if its binding name marks it as an S3 method, an
/// R lifecycle hook, or its body is an S3/S4 generic dispatcher (`UseMethod`
/// / `standardGeneric`). All three patterns have signatures dictated by
/// something other than the function body, so unused params aren't a bug.
fn should_skip_function(
    info: &SemanticInfo<'_>,
    scope_id: ScopeId,
    exports: &HashSet<String>,
) -> bool {
    if let Some(name) = info.function_binding_name(scope_id)
        && (is_package_hook(&name) || is_s3_method(&name, exports))
    {
        return true;
    }
    if let Some(func_def) = info.function_definition(scope_id)
        && let Some(func) = func_def.cast::<RFunctionDefinition>()
        && is_dispatch_generic(&func)
    {
        return true;
    }
    false
}

/// Package hooks have signatures fixed by the R runtime. Names mirrored from
/// `unused_function`'s allow-list.
fn is_package_hook(name: &str) -> bool {
    matches!(
        name,
        ".onLoad"
            | "on_load"
            | ".onAttach"
            | ".onDetach"
            | ".onUnload"
            | ".Last.lib"
            | ".First.lib"
    )
}

/// Treat `generic.class`-style exported names as S3 methods. Relies on the
/// NAMESPACE having an `S3method()` directive (which `parse_namespace_exports`
/// folds into the export set). A non-method with a `.` in its name that's
/// also exported (e.g. `do.thing`) gets the same free pass — acceptable
/// trade-off for the simpler heuristic.
fn is_s3_method(name: &str, exports: &HashSet<String>) -> bool {
    name.contains('.') && exports.contains(name)
}

/// True if the function body contains a top-level call to `UseMethod()` or
/// `standardGeneric()` — the canonical S3/S4 generic dispatcher.
fn is_dispatch_generic(func: &RFunctionDefinition) -> bool {
    let Ok(body) = func.body() else {
        return false;
    };
    if let Some(braced) = body.as_r_braced_expressions() {
        return braced
            .expressions()
            .into_iter()
            .any(|e| is_dispatch_call(&e));
    }
    is_dispatch_call(&body)
}

fn is_dispatch_call(expr: &AnyRExpression) -> bool {
    let AnyRExpression::RCall(call) = expr else {
        return false;
    };
    let Ok(func) = call.function() else {
        return false;
    };
    let AnyRExpression::RIdentifier(id) = func else {
        return false;
    };
    let name = id.name_text();
    name == "UseMethod" || name == "standardGeneric"
}

fn is_skipped_parameter_name(name: &str) -> bool {
    // `...` is the variadic parameter; it's never "unused" in any meaningful
    // sense — callers pass arguments through it, the body forwards it.
    name == "..."
}

/// Narrow the parameter range to just the identifier, excluding any
/// default-value clause (`= …`). Oak's `Parameter(node)` carries the whole
/// `R_PARAMETER` syntax.
fn parameter_name_range(node: &RSyntaxNode) -> Option<TextRange> {
    node.children()
        .find(|c| c.kind() == RSyntaxKind::R_IDENTIFIER || c.kind() == RSyntaxKind::R_DOTS)
        .map(|c| c.text_trimmed_range())
}

fn make_diagnostic(name: &str, range: TextRange) -> Diagnostic {
    Diagnostic::new(
        ViolationData::new(
            "unused_argument".to_string(),
            format!("Argument `{name}` is defined but never used."),
            None,
        ),
        range,
        Fix::empty(),
    )
}
