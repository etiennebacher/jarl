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
/// ## R Markdown and Quarto
///
/// The chunks of an `.Rmd`/`.qmd` document run one after another in a single R
/// session, so a `stop()` in one chunk does make the code in the chunks after
/// it unreachable. This is not the case if at least one the following two options
/// are specified:
///
/// - `eval = FALSE` (or `#| eval: false`), because the chunk never runs at all;
/// - `error = TRUE` (or `#| error: true`), because the chunk prints the error
///   but continues to evaluate the subsequent chunks.
///
/// Such a chunk is left out of the analysis entirely, so the following also
/// wouldn't be reported:
///
/// ````markdown
/// ```
/// #| eval: false
/// stop("a")
/// 1 + 1 # unreachable but not reported
/// ```
/// ````
///
/// Note that unreachable code *not at the top-level* would still be reported,
/// e.g.:
///
/// ````markdown
/// ```
/// #| eval: false
/// f <- function() {
///   stop("a")
///   1 + 1 # reported as unreachable
/// }
/// ```
/// ````
///
/// A chunk whose evaluation is decided at render time (e.g. `#| eval: run_it`)
/// is considered evaluated.
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

    // Build the control flow graph for top-level code
    let stopping = &checker.rule_options.unreachable_code.stopping_functions;
    let cfg = build_cfg_top_level(&running_expressions(expressions, checker), stopping);

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

    Ok(diagnostics)
}

/// The top-level expressions that take part in the file's control flow.
///
/// Every expression of an R script does. An Rmd/Qmd document is different,
/// because two chunk options take a chunk out of the flow:
///
/// - `eval = FALSE` — the chunk never runs, so nothing in it can be reached
///   and nothing in it can stop a later chunk. Nothing to say about it either
///   way: reporting one line of a chunk that doesn't run, because a line above
///   it wouldn't have returned, singles out an arbitrary line.
/// - `error = TRUE` — the chunk runs, but knitr prints the condition and keeps
///   going, through the rest of the chunk as well as the rest of the document,
///   so nothing in it ends control flow anywhere.
///
/// Leaving a chunk out is not the same as cutting the document in two: a
/// `stop()` before one still makes the code after it unreachable.
///
/// Code inside a function definition is unaffected either way: a `return()`
/// still ends the function it is written in, and `unreachable_code` analyzes
/// that body separately.
fn running_expressions(expressions: &[RSyntaxNode], checker: &Checker) -> Vec<RSyntaxNode> {
    if checker.chunks.is_empty() {
        return expressions.to_vec();
    }

    expressions
        .iter()
        .filter(
            |expression| match checker.chunk_index_at(expression.text_trimmed_range()) {
                Some(index) => {
                    let options = checker.chunks[index].options;
                    options.eval && !options.error
                }
                None => true,
            },
        )
        .cloned()
        .collect()
}
