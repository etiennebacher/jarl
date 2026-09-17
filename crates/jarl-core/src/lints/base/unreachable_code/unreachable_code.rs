use crate::checker::Checker;
use crate::diagnostic::*;
use air_r_syntax::*;

use super::cfg::{UnreachableReason, build_cfg, build_cfg_top_level, find_unreachable_code};
use crate::rule_set::Rule;

/// Version added: 0.4.0
///
/// ## What it does
///
/// Detects code that can never be executed because it appears after control
/// flow statements like `return`, `break`, or `next`, or in branches that
/// cannot be reached.
///
/// ## Why is this bad?
///
/// Unreachable code indicates a logic error or dead code that should be removed.
/// It clutters the codebase, confuses readers, and may indicate unintended behavior.
///
/// ## Examples
///
/// ```r
/// if (x > 5) {
///   stop("hi")
/// } else {
///   stop("bye")
/// }
/// 1 + 1 # unreachable
/// ```
///
/// ```r
/// foo <- function(x) {
///   return(x + 1)
///   print("hi")  # unreachable
/// }
/// ```
///
/// ```r
/// foo <- function(x) {
///   for (i in 1:10) {
///     x <- x + 1
///     if (x > 10) {
///        break
///        print("x is greater than 10") # unreachable
///     }
///   }
/// }
/// ```
///
/// ## R Markdown and Quarto
///
/// The chunks of an `.Rmd`/`.qmd` document run one after another in a single R
/// session, so a `stop()` in one chunk does make the code in the chunks after
/// it unreachable. Two chunk options break that chain, and nothing after a
/// chunk carrying one is reported:
///
/// - `eval = FALSE` (or `#| eval: false`), because the chunk never runs at all;
/// - `error = TRUE` (or `#| error: true`), because knitr prints the condition
///   and carries on rendering — through the rest of that chunk as well as the
///   rest of the document.
///
/// An option whose value is decided at render time (`eval = run_it`) is read as
/// the ordinary case, so the document keeps stopping.
pub fn unreachable_code(
    ast: &RFunctionDefinition,
    checker: &Checker,
) -> anyhow::Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    // Build the control flow graph for this function
    let stopping = &checker.rule_options.unreachable_code.stopping_functions;
    let cfg = build_cfg(ast, stopping);

    // Find all unreachable code
    for unreachable_info in find_unreachable_code(&cfg) {
        let diagnostic = Diagnostic::new(
            ViolationData::new(
                Rule::UnreachableCode,
                unreachable_info.reason.message().to_string(),
                None,
            ),
            unreachable_info.range,
            Fix::empty(),
        );
        diagnostics.push(diagnostic);
    }

    Ok(diagnostics)
}

/// Detect unreachable code in top-level R code
///
/// This function is similar to `unreachable_code` but is designed for top-level code.
/// It filters out certain unreachable reasons that don't make sense at the top level:
/// - `AfterReturn` is ignored (can't return from top-level)
/// - `NoPathFromEntry` is ignored (doesn't make sense at top level)
pub fn unreachable_code_top_level(
    expressions: &[RSyntaxNode],
    checker: &Checker,
) -> anyhow::Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let stopping = &checker.rule_options.unreachable_code.stopping_functions;

    for run in flow_runs(expressions, checker) {
        // Build the control flow graph for top-level code
        let cfg = build_cfg_top_level(&run, stopping);

        // Find all unreachable code
        for unreachable_info in find_unreachable_code(&cfg) {
            // Filter out reasons that don't make sense at top level
            if matches!(
                unreachable_info.reason,
                UnreachableReason::AfterReturn | UnreachableReason::NoPathFromEntry
            ) {
                continue;
            }

            let diagnostic = Diagnostic::new(
                ViolationData::new(
                    Rule::UnreachableCode,
                    unreachable_info.reason.message().to_string(),
                    None,
                ),
                unreachable_info.range,
                Fix::empty(),
            );
            diagnostics.push(diagnostic);
        }
    }

    Ok(diagnostics)
}

/// Split the top-level expressions into the runs that share a control flow.
///
/// An R script is a single run: every expression can stop the ones after it.
/// An Rmd/Qmd document is not, because two chunk options break the chain:
///
/// - `eval = FALSE` — the chunk never runs, so a `stop()` in it ends nothing
///   in the document. Its statements still follow one another as written, so
///   the chunk becomes a run of its own and is analyzed on its own terms.
/// - `error = TRUE` — the chunk runs, but knitr prints the condition and keeps
///   going, through the rest of the chunk as well as the rest of the document.
///   Nothing there can end control flow anywhere, so the chunk is left out of
///   every run.
///
/// Code inside a function definition is unaffected either way: a `return()`
/// still ends the function it is written in, and `unreachable_code` analyzes
/// that body separately.
fn flow_runs(expressions: &[RSyntaxNode], checker: &Checker) -> Vec<Vec<RSyntaxNode>> {
    if checker.chunks.is_empty() {
        return vec![expressions.to_vec()];
    }

    let mut document: Vec<RSyntaxNode> = Vec::new();
    let mut unevaluated: Vec<Vec<RSyntaxNode>> = vec![Vec::new(); checker.chunks.len()];

    for expression in expressions {
        let range = expression.text_trimmed_range();
        let Some(index) = checker.chunk_index_at(range) else {
            document.push(expression.clone());
            continue;
        };
        let options = checker.chunks[index].options;
        if options.error {
            continue;
        }
        if options.eval {
            document.push(expression.clone());
        } else {
            unevaluated[index].push(expression.clone());
        }
    }

    std::iter::once(document)
        .chain(unevaluated)
        .filter(|run| !run.is_empty())
        .collect()
}
