use crate::diagnostic::*;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct EqualAssignment;

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
/// * https://style.tidyverse.org/syntax.html#assignment-1
/// * https://stackoverflow.com/a/1742550
impl Violation for EqualAssignment {
    fn name(&self) -> String {
        "assignment".to_string()
    }
    fn body(&self) -> String {
        "Use <- for assignment.".to_string()
    }
}

pub fn assignment(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let operator = operator?;
    let lhs = left?.into_syntax();
    let rhs = right?.into_syntax();

    if operator.kind() != RSyntaxKind::EQUAL && operator.kind() != RSyntaxKind::ASSIGN_RIGHT {
        return Ok(None);
    };

    // We don't want the reported range to be the entire binary expression. The
    // range is used in the LSP to highlight lints, but highlighting the entire
    // binary expression would be super annoying for long functions that are
    // assigned using `=`.
    let range_to_report: TextRange;

    let replacement = match operator.kind() {
        RSyntaxKind::EQUAL => {
            range_to_report = TextRange::new(
                lhs.text_trimmed_range().start(),
                operator.text_trimmed_range().end(),
            );
            format!("{} <- {}", lhs.text_trimmed(), rhs.text_trimmed())
        }
        RSyntaxKind::ASSIGN_RIGHT => {
            range_to_report = TextRange::new(
                operator.text_trimmed_range().start(),
                rhs.text_trimmed_range().end(),
            );
            format!("{} <- {}", rhs.text_trimmed(), lhs.text_trimmed())
        }
        _ => unreachable!(),
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        EqualAssignment,
        range_to_report,
        Fix {
            content: replacement,
            start: range.start().into(),
            end: range.end().into(),
        },
    );

    Ok(Some(diagnostic))
}
