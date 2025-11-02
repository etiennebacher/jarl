use crate::diagnostic::*;
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for usage of `=` as assignment operator.
///
/// ## Why is this bad?
///
/// This is not "bad" strictly speaking since in most cases using `=` and `<-`
/// is equivalent. Some very popular packages use `=` without problems.
///
/// Nonetheless, `<-` is more popular and this rule may be useful to avoid
/// mixing both operators in a codebase.
///
/// ## Example
///
/// ```r
/// x = "a"
/// ```
///
/// Use instead:
/// ```r
/// x <- "a"
/// ```
///
/// ## References
///
/// See:
///
/// - [https://style.tidyverse.org/syntax.html#assignment-1](https://style.tidyverse.org/syntax.html#assignment-1)
/// - [https://stackoverflow.com/a/1742550](https://stackoverflow.com/a/1742550)
pub fn assignment(
    ast: &RBinaryExpression,
    assignment_op: RSyntaxKind,
) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let operator = operator?;
    let lhs = left?.into_syntax();
    let rhs = right?.into_syntax();

    let operator_to_check = match assignment_op {
        RSyntaxKind::ASSIGN => RSyntaxKind::EQUAL,
        RSyntaxKind::EQUAL => RSyntaxKind::ASSIGN,
        _ => unreachable!(),
    };

    if operator.kind() != operator_to_check && operator.kind() != RSyntaxKind::ASSIGN_RIGHT {
        return Ok(None);
    };

    // We don't want the reported range to be the entire binary expression. The
    // range is used in the LSP to highlight lints, but highlighting the entire
    // binary expression would be super annoying for long functions that are
    // assigned using `=`.
    let (range_to_report, msg, replacement) = match operator.kind() {
        RSyntaxKind::EQUAL => {
            let range = TextRange::new(
                lhs.text_trimmed_range().start(),
                operator.text_trimmed_range().end(),
            );
            let message = "Use `<-` for assignment.";
            let fix = format!("{} <- {}", lhs.text_trimmed(), rhs.text_trimmed());
            (range, message, fix)
        }
        RSyntaxKind::ASSIGN => {
            let range = TextRange::new(
                lhs.text_trimmed_range().start(),
                operator.text_trimmed_range().end(),
            );
            let message = "Use `=` for assignment.";
            let fix = format!("{} = {}", lhs.text_trimmed(), rhs.text_trimmed());
            (range, message, fix)
        }
        RSyntaxKind::ASSIGN_RIGHT => {
            let range = TextRange::new(
                operator.text_trimmed_range().start(),
                rhs.text_trimmed_range().end(),
            );
            let (message, fix) = match assignment_op {
                RSyntaxKind::ASSIGN => {
                    let msg = "Use `<-` for assignment.";
                    let replacement = format!("{} <- {}", rhs.text_trimmed(), lhs.text_trimmed());
                    (msg, replacement)
                }
                RSyntaxKind::EQUAL => {
                    let msg = "Use `=` for assignment.";
                    let replacement = format!("{} = {}", rhs.text_trimmed(), lhs.text_trimmed());
                    (msg, replacement)
                }
                _ => unreachable!(),
            };
            (range, message, fix)
        }
        _ => unreachable!(),
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new("assignment".to_string(), msg.to_string(), None),
        range_to_report,
        Fix {
            content: replacement,
            start: range.start().into(),
            end: range.end().into(),
            to_skip: false,
        },
    );

    Ok(Some(diagnostic))
}
