use crate::diagnostic::{Diagnostic, Fix, Violation};
use air_r_syntax::RParenthesizedExpression;
use biome_rowan::AstNode;

pub struct UnnecessaryParentheses {
    count: usize,
}

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
impl Violation for UnnecessaryParentheses {
    fn name(&self) -> String {
        "unnecessary_parentheses".to_string()
    }

    fn body(&self) -> String {
        format!(
            "This expression contains {} unnecessary {} of parentheses.",
            self.count,
            if self.count == 1 { "pair" } else { "pairs" },
        )
    }

    fn suggestion(&self) -> Option<String> {
        Some(format!(
            "Remove {} {} of parentheses.",
            self.count,
            if self.count == 1 { "pair" } else { "pairs" },
        ))
    }
}

pub fn unnecessary_parentheses(
    ast: &RParenthesizedExpression,
) -> anyhow::Result<Option<Diagnostic>> {
    if ast
        .syntax()
        .parent()
        .and_then(RParenthesizedExpression::cast)
        .is_some()
    {
        return Ok(None);
    }

    let mut count = 0;
    let mut current = ast.body()?;

    while let Some(inner) = current.as_r_parenthesized_expression() {
        count += 1;
        current = inner.body()?;
    }

    if count == 0 {
        return Ok(None);
    }

    Ok(Some(Diagnostic::new(
        UnnecessaryParentheses { count },
        ast.syntax().text_trimmed_range(),
        Fix::empty(),
    )))
}
