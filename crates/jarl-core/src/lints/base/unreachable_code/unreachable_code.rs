use crate::diagnostic::*;
use air_r_syntax::*;

use super::cfg::{UnreachableReason, build_cfg, build_cfg_top_level, find_unreachable_code};

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
pub fn unreachable_code(ast: &RFunctionDefinition) -> anyhow::Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    // Build the control flow graph for this function
    let cfg = build_cfg(ast);

    // Find all unreachable code
    for unreachable_info in find_unreachable_code(&cfg) {
        let diagnostic = Diagnostic::new(
            ViolationData::new(
                "unreachable_code".to_string(),
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
pub fn unreachable_code_top_level(expressions: &[RSyntaxNode]) -> anyhow::Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    // Build the control flow graph for top-level code
    let cfg = build_cfg_top_level(expressions);

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
                "unreachable_code".to_string(),
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
