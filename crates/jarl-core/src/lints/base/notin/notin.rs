use crate::diagnostic::*;
use crate::utils::node_contains_comments;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct Notin;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for usage of `!(x %in% y)` and recommends using `%notin%` instead.
///
/// ## Why is this bad?
///
/// Starting from R 4.6.0, the `%notin%` operator is available in base R.
/// Using `%notin%` makes the intent clearer than wrapping `%in%` in a negation.
///
/// ## Example
///
/// ```r
/// if (!(x %in% choices)) {
///   print("x is not in choices")
/// }
/// ```
///
/// Use instead:
/// ```r
/// if (x %notin% choices) {
///   print("x is not in choices")
/// }
/// ```
///
/// ## References
///
/// See `?match`
impl Violation for Notin {
    fn name(&self) -> String {
        "notin".to_string()
    }
    fn body(&self) -> String {
        "`!(x %in% y)` can be simplified.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `x %notin% y` instead.".to_string())
    }
}

pub fn notin(ast: &RUnaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let operator = ast.operator()?;

    // Ensure the operator is `!`
    if operator.kind() != RSyntaxKind::BANG {
        return Ok(None);
    }

    // Ensure the operand is a parenthesized expression
    let argument = ast.argument()?;
    let paren_expr = unwrap_or_return_none!(argument.as_r_parenthesized_expression());
    let body = paren_expr.body()?;
    let binary_expression = unwrap_or_return_none!(body.as_r_binary_expression());

    // Ensure the binary expression is of the form `x %in% y`
    let RBinaryExpressionFields { left, operator, right } = binary_expression.as_fields();
    let left = left?;
    let operator = operator?;
    let right = right?;

    if operator.kind() != RSyntaxKind::SPECIAL || operator.text_trimmed() != "%in%" {
        return Ok(None);
    }

    // Skip if either operand is `NA`, process `NA` in other rules
    if left.as_r_na_expression().is_some() || right.as_r_na_expression().is_some() {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        Notin,
        range,
        Fix {
            content: format!(
                "{} %notin% {}",
                left.to_trimmed_text(),
                right.to_trimmed_text()
            ),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
