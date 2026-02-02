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
/// Unreachable code can only be detected in functions.
///
/// ## Example
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
/// ```r
/// foo <- function(x) {
///   if (x > 5) {
///     return("hi")
///   } else {
///     return("bye")
///   }
///   1 + 1 # unreachable
/// }
/// ```
pub fn unreachable_code(ast: &RFunctionDefinition) -> anyhow::Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    // Build the control flow graph for this function
    let cfg = build_cfg(ast);

    // Find all unreachable code
    let unreachable_blocks = find_unreachable_code(&cfg);

    for unreachable_info in unreachable_blocks {
        let message = match unreachable_info.reason {
            UnreachableReason::AfterReturn => {
                "This code is unreachable because it appears after a return statement."
            }
            UnreachableReason::AfterStop => {
                "This code is unreachable because it appears after a `stop()` statement (or equivalent)."
            }
            UnreachableReason::AfterBreak => {
                "This code is unreachable because it appears after a break statement."
            }
            UnreachableReason::AfterNext => {
                "This code is unreachable because it appears after a next statement."
            }
            UnreachableReason::AfterBranchTerminating => {
                "This code is unreachable because the preceding if/else terminates in all branches."
            }
            UnreachableReason::DeadBranch => "This code is in a branch that can never be executed.",
            UnreachableReason::NoPathFromEntry => {
                "This code has no execution path from the function entry."
            }
        };

        let diagnostic = Diagnostic::new(
            ViolationData::new("unreachable_code".to_string(), message.to_string(), None),
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
/// - `AfterStop` and `AfterBranchTerminating` are only reported if they occur in nested contexts (nesting_level > 0)
/// - `AfterBreak`, `AfterNext`, and `DeadBranch` are always reported
pub fn unreachable_code_top_level(expressions: &[RSyntaxNode]) -> anyhow::Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    // Build the control flow graph for top-level code
    let cfg = build_cfg_top_level(expressions);

    // Find all unreachable code
    let unreachable_blocks = find_unreachable_code(&cfg);

    for unreachable_info in unreachable_blocks {
        // Filter based on reason and nesting level
        let should_report = match unreachable_info.reason {
            // Always ignore these at top level
            UnreachableReason::AfterReturn | UnreachableReason::NoPathFromEntry => false,

            // Always report these
            UnreachableReason::AfterBreak
            | UnreachableReason::AfterNext
            | UnreachableReason::DeadBranch
            | UnreachableReason::AfterStop
            | UnreachableReason::AfterBranchTerminating => true,
        };

        if !should_report {
            continue;
        }

        let message = match unreachable_info.reason {
            UnreachableReason::AfterReturn => {
                "This code is unreachable because it appears after a return statement."
            }
            UnreachableReason::AfterStop => {
                "This code is unreachable because it appears after a `stop()` statement (or equivalent)."
            }
            UnreachableReason::AfterBreak => {
                "This code is unreachable because it appears after a break statement."
            }
            UnreachableReason::AfterNext => {
                "This code is unreachable because it appears after a next statement."
            }
            UnreachableReason::AfterBranchTerminating => {
                "This code is unreachable because the preceding if/else terminates in all branches."
            }
            UnreachableReason::DeadBranch => {
                "This code is in a branch that can never be executed due to a constant condition."
            }
            UnreachableReason::NoPathFromEntry => {
                "This code has no execution path from the function entry."
            }
        };

        let diagnostic = Diagnostic::new(
            ViolationData::new("unreachable_code".to_string(), message.to_string(), None),
            unreachable_info.range,
            Fix::empty(),
        );

        diagnostics.push(diagnostic);
    }

    Ok(diagnostics)
}
