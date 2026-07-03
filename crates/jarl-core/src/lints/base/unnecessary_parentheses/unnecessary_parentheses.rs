use crate::diagnostic::{Diagnostic, Fix, ViolationData};
use crate::utils::node_contains_comments;
use air_r_syntax::RParenthesizedExpression;
use biome_rowan::AstNode;

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

    let mut count = 1;
    let mut current = ast.body()?;

    // we count the number of nested unnecessary parentheses
    while let Some(inner) = current.as_r_parenthesized_expression() {
        count += 1;
        current = inner.body()?;
    }

    if count == 1 {
        return Ok(None);
    }

    // We only remove the redundant pairs and always keep the innermost one,
    // so the fix never depends on the surrounding context.
    let removable_count = count - 1;

    let (body, suggestion) = if removable_count == 1 {
        (
            "This expression contains an unnecessary pair of parentheses.".to_string(),
            "Remove the unnecessary pair of parentheses.".to_string(),
        )
    } else {
        (
            format!("This expression contains {removable_count} unnecessary pairs of parentheses."),
            format!("Remove {removable_count} pairs of parentheses."),
        )
    };

    let range = ast.syntax().text_trimmed_range();

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            "unnecessary_parentheses".to_string(),
            body,
            Some(suggestion),
        ),
        range,
        Fix {
            content: format!("({})", current.to_trimmed_string()),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    )))
}
