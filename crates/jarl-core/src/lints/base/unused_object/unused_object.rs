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
/// ## Features
///
/// Apart from the standard usage of objects in R (e.g. `x <- 1; print(x)`),
/// this rule handles the following cases:
///
/// - String interpolation in the `glue`, `cli`, and `stringr` packages, e.g.
///   in this case `x` is not reported as unused:
///
///   ```r
///   x <- 1
///   glue::glue("{print(x)}")
///   ```
///
///   Custom functions or functions from other packages providing string
///   interpolation are not supported.
///
/// - The `%<>%` operator from `magrittr` is supported.
///
/// - Explicit cross-file analysis: calls to `source()` or `targets::tar_source()`
///   are detected, e.g. this doesn't report `x` as unused:
///
///     - `foo.R`:
///       ```r
///       x <- 1
///       source("bar.R")
///       ```
///
///     - `bar.R`:
///       ```r
///       print(x)
///       ```
///
///   Similarly, the definition could be made in the sourced file (`bar.R`) and
///   the use could be made in the other file (`foo.R`).
///
/// - Implicit cross-file analysis. All files in an `R` folder (whether this
///   corresponds to an R package or to another project type) are collated and
///   share the same namespace, meaning that an object defined in `R/a.R` could
///   seamlessly be detected as used in `R/b.R`.
///
/// - Some functions that can call other quoted functions (e.g. `do.call()`) are
///   supported.
///
/// ## Limitations
///
/// Some cases are deliberately left aside or might be tackled in the future:
///
/// - Some functions such as `get()` or `mget()` are not handled.
///
/// - Quoted code that is evaluated later may lead to false positives, e.g. this
///   would wrongly report `x` as unused:
///
///   ```r
///   x <- 1
///   e <- quote(x + 1)
///   eval(e)
///   ```
///
/// - `source()` and alike only accept literal paths, not R objects, e.g. this
///   isn't handled by Jarl:
///
///   ```r
///   for (i in my_paths) source(i)
///   ```
///
/// ## In R Markdown and Quarto files
///
/// Jarl bundles all chunks together before running the analysis, meaning that
/// `unused_object` would properly detect whether an object created in a chunk
/// is used in another.
///
/// There are two other cases to handle:
///
/// - objects that are present in a chunk with `eval = FALSE` or `#| eval: false`
///   are not marked as "used". For instance, in the following example, the
///   object `x` would be reported as unused:
///
///   ````markdown
///   ```{{r}}
///   x <- 1
///   ```
///
///   ```{{r eval = FALSE}}
///   print(x)
///   ```
///   ````
///
///   Note that if the option value is only available at runtime (e.g.
///   `eval = my_r_object`) then Jarl assumes that the chunk is evaluated.
///
/// - inline R code is taken into account, `` `r x` `` in the text would keep
///   `x` from being reported as unused.
///
/// ## In roxygen examples
///
/// R code in `@examples` and `@examplesIf` sections is checked too, in files
/// under `R/` in a package. This can be turned off with `check-roxygen`.
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
    let info = SemanticInfo::build(
        &root,
        expressions,
        semantic,
        &checker.source_index_cache,
        &checker.loaded_packages,
        &checker.unevaluated_ranges,
    );
    let exports = &checker.namespace_exports;

    let mut diagnostics = Vec::new();
    let top_level = ScopeId::from(0);
    for scope_id in info.scope_ids() {
        for (def_id, def) in semantic.definitions(scope_id).iter() {
            if !should_lint_definition(&info, def) {
                continue;
            }
            if info.is_definition_used(scope_id, def_id) {
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

    // An assignment in `within(data, { … })` mutates the environment the call
    // returns, so the binding is the result rather than a dead store.
    if info.is_in_returned_env(def.range()) {
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
