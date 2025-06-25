use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;
use anyhow::{Context, Result};

pub struct AnyDuplicated;

/// ## What it does
///
/// Checks for usage of `any(duplicated(...))`.
///
/// ## Why is this bad?
///
/// `any(duplicated(...))` is valid code but requires the evaluation of
/// `duplicated()` on the entire input first.
///
/// There is a more efficient function in base R called `anyDuplicated()` that
/// is more efficient, both in speed and memory used. `anyDuplicated()` returns
/// the index of the first duplicated value, or 0 if there is none.
///
/// Therefore, we can replace `any(duplicated(...))` by `anyDuplicated(...) > 0`.
///
/// ## Example
///
/// ```r
/// x <- c(1:10000, 1, NA)
/// any(duplicated(x))
/// ```
///
/// Use instead:
/// ```r
/// x <- c(1:10000, 1, NA)
/// anyDuplicated(x) > 0
/// ```
///
/// ## References
///
/// See `?anyDuplicated`
impl Violation for AnyDuplicated {
    fn name(&self) -> String {
        "any-duplicated".to_string()
    }
    fn body(&self) -> String {
        "`any(duplicated(...))` is inefficient. Use `anyDuplicated(...) > 0` instead.".to_string()
    }
}

impl LintChecker for AnyDuplicated {
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

            if fun.text_trimmed() == "duplicated" && fun.kind() == RSyntaxKind::R_IDENTIFIER {
                let range = ast.text_trimmed_range();
                diagnostics.push(Diagnostic::new(
                    AnyDuplicated,
                    file,
                    range,
                    Fix {
                        content: format!("anyDuplicated({}) > 0", fun_content),
                        start: range.start().into(),
                        end: range.end().into(),
                    },
                ))
            }
        }
        Ok(diagnostics)
    }
}
