use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;
use anyhow::{Context, Result};

pub struct LengthLevels;

/// ## What it does
///
/// Check for `length(levels(...))` and replace it with `nlevels(...)`.
///
/// ## Why is this bad?
///
/// `length(levels(...))` is harder to read `nlevels(...)`.
///
/// Internally, `nlevels()` calls `length(levels(...))` so there are no
/// performance gains.
///
/// ## Example
///
/// ```r
/// x <- factor(1:3)
/// length(levels(x))
/// ```
///
/// Use instead:
/// ```r
/// x <- factor(1:3)
/// nlevels(x)
/// ```
impl Violation for LengthLevels {
    fn name(&self) -> String {
        "length_levels".to_string()
    }
    fn body(&self) -> String {
        "Use `nlevels(...)` instead of `length(levels(...))`.".to_string()
    }
}

impl LintChecker for LengthLevels {
    fn check(&self, ast: &RSyntaxNode, file: &str) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        if ast.kind() != RSyntaxKind::R_CALL {
            return Ok(diagnostics);
        }
        let call = ast
            .first_child()
            .context("Couldn't find function name")?
            .text_trimmed();

        if call != "length" {
            return Ok(diagnostics);
        }

        let unnamed_arg = ast.descendants().find(|x| {
            x.kind() == RSyntaxKind::R_ARGUMENT
                && x.first_child()
                    .map(|child| child.kind() != RSyntaxKind::R_ARGUMENT_NAME_CLAUSE)
                    .unwrap_or(false)
        });

        let y = unnamed_arg
            .unwrap()
            .first_child()
            .context("No first child found")?;

        if y.kind() == RSyntaxKind::R_CALL {
            let fun = y.first_child().context("No function found")?;
            let fun_content = y
                .children()
                .nth(1)
                .context("Internal error")?
                .first_child()
                .context("Internal error")?
                .text();

            if fun.text_trimmed() == "levels" && fun.kind() == RSyntaxKind::R_IDENTIFIER {
                let range = ast.text_trimmed_range();
                diagnostics.push(Diagnostic::new(
                    LengthLevels,
                    file,
                    range,
                    Fix {
                        content: format!("nlevels({})", fun_content),
                        start: range.start().into(),
                        end: range.end().into(),
                    },
                ))
            }
        }
        Ok(diagnostics)
    }
}
