use crate::diagnostic::*;
use air_r_syntax::*;
use biome_rowan::AstNode;

use super::cfg::{UnreachableReason, build_cfg, find_unreachable_code};
use crate::diagnostic::Fix;

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
/// ## Example
///
/// ```r
/// foo <- function(x) {
///   return(x + 1)
///   print("This will never execute")  # unreachable
/// }
/// ```
///
/// ```r
/// for (i in 1:10) {
///   break
///   x <- i  # unreachable
/// }
/// ```

/// Analyze a function for unreachable code using control flow graph analysis
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
            UnreachableReason::AfterBreak => {
                "This code is unreachable because it appears after a break statement."
            }
            UnreachableReason::AfterNext => {
                "This code is unreachable because it appears after a next statement."
            }
            UnreachableReason::DeadBranch => "This code is in a branch that can never be executed.",
            UnreachableReason::NoPathFromEntry => {
                "This code has no execution path from the function entry."
            }
        };

        let diagnostic = Diagnostic::new(
            ViolationData::new(
                "unreachable_code".to_string(),
                message.to_string(),
                Some("".to_string()),
            ),
            unreachable_info.range,
            Fix::empty(),
        );

        diagnostics.push(diagnostic);
    }

    Ok(diagnostics)
}

// /// Entry point for analyzing statements and expressions for unreachable code
// /// This traverses the AST and builds CFGs for each function definition
// pub fn check_unreachable_code(node: &RSyntaxNode) -> anyhow::Result<Vec<Diagnostic>> {
//     let mut diagnostics = Vec::new();

//     // Find all function definitions in the tree
//     for func_node in node.descendants() {
//         if let Some(func_def) = RFunctionDefinition::cast(func_node) {
//             match unreachable_code(&func_def) {
//                 Ok(mut func_diagnostics) => {
//                     diagnostics.append(&mut func_diagnostics);
//                 }
//                 Err(e) => {
//                     // Log error but continue processing other functions
//                     eprintln!("Error analyzing function for unreachable code: {}", e);
//                 }
//             }
//         }
//     }

//     Ok(diagnostics)
// }
