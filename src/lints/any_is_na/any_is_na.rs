use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;
use anyhow::{Context, Result};

pub struct AnyIsNa;

/// ## What it does
///
/// Checks for usage of `any(is.na(...))`.
///
/// ## Why is this bad?
///
/// `any(is.na(...))` is valid code but requires the evaluation of `is.na()` on
/// the entire input first.
///
/// There is a more efficient function in base R called `anyNA()` that is more
/// efficient, both in speed and memory used.
///
/// ## Example
///
/// ```r
/// x <- c(1:10000, NA)
/// any(is.na(x))
/// ```
///
/// Use instead:
/// ```r
/// x <- c(1:10000, NA)
/// anyNA(x)
/// ```
///
/// ## References
///
/// See `?anyNA`
impl Violation for AnyIsNa {
    fn name(&self) -> String {
        "any-na".to_string()
    }
    fn body(&self) -> String {
        "`any(is.na(...))` is inefficient. Use `anyNA(...)` instead.".to_string()
    }
}

impl LintChecker for AnyIsNa {
    fn check(&self, ast: &RSyntaxNode, file: &str) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        if ast.kind() != RSyntaxKind::R_CALL {
            return Ok(diagnostics);
        }
        let call = ast
            .first_child()
            .context("Couldn't find function name")?
            .text_trimmed();

        if call != "any" {
            return Ok(diagnostics);
        }

        let unnamed_arg = ast.descendants().find(|x| {
            x.kind() == RSyntaxKind::R_ARGUMENT
                && x.first_child()
                    .map(|child| child.kind() != RSyntaxKind::R_ARGUMENT_NAME_CLAUSE)
                    .unwrap_or(false)
        });

        // any(na.rm = TRUE/FALSE) and any() are valid
        if unnamed_arg.is_none() {
            return Ok(diagnostics);
        }

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

            if fun.text_trimmed() == "is.na" && fun.kind() == RSyntaxKind::R_IDENTIFIER {
                let range = ast.text_trimmed_range();
                diagnostics.push(Diagnostic::new(
                    AnyIsNa,
                    file,
                    range,
                    Fix {
                        content: format!("anyNA({})", fun_content),
                        start: range.start().into(),
                        end: range.end().into(),
                    },
                ))
            }
        }
        Ok(diagnostics)
    }
}
