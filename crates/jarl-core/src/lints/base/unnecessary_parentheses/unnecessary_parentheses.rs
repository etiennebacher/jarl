use crate::diagnostic::{Diagnostic, Fix, Violation};
use air_r_syntax::RParenthesizedExpression;
use biome_rowan::AstNode;

pub struct UnnecessaryParenthesis;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for expressions wrapped in multiple pairs of parentheses.
///
/// ## Why is this bad?
///
/// Repeated parentheses do not change the meaning of the expression and can make
/// the code harder to read.
///
/// ## Example
///
/// ```r
/// ((x + 1))
/// ```
///
/// Use instead:
///
/// ```r
/// (x + 1)
/// ```
impl Violation for UnnecessaryParenthesis {
    fn name(&self) -> String {
        "unnecessary_parentheses".to_string()
    }

    fn body(&self) -> String {
        "This expression contains unnecessary parentheses.".to_string()
    }

    fn suggestion(&self) -> Option<String> {
        Some("Remove one pair of parentheses.".to_string())
    }
}

pub fn unnecessary_parentheses(
    ast: &RParenthesizedExpression,
) -> anyhow::Result<Option<Diagnostic>> {
    if ast.body()?.as_r_parenthesized_expression().is_none() {
        return Ok(None);
    }

    Ok(Some(Diagnostic::new(
        UnnecessaryParenthesis,
        ast.syntax().text_trimmed_range(),
        Fix::empty(),
    )))
}
