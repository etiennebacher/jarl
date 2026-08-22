use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::node_contains_comments;
use air_r_syntax::*;
use biome_rowan::AstNode;

/// Version added: 0.0.23
///
/// ## What it does
///
/// Checks for patterns similar to `!(... < ...)`.
///
/// ## Why is this bad?
///
/// This pattern may be hard to read and could be simplified by removing the `!`
/// operator and inverting the operator (e.g. `<` would become `>=`).
///
/// This rule has an unsafe fix because of operator precedence around the
/// comparison:
///
/// ```r
/// x <- 1
/// y <- 2
///
/// 2 * !(x < y)
/// #> [1] 0
/// 2 * x >= y
/// #> [1] TRUE
/// ```
///
/// ## Example
///
/// ```r
/// !(x < y + 1)
/// !(x == y + 1)
/// ```
///
/// Use instead:
/// ```r
/// x >= y + 1
/// x != y + 1
/// ```
pub fn comparison_negation(ast: &RUnaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let operator = ast.operator()?;

    if operator.kind() != RSyntaxKind::BANG {
        return Ok(None);
    }

    let argument = ast.argument()?;

    let paren_expr = unwrap_or_return_none!(argument.as_r_parenthesized_expression());

    let body = paren_expr.body()?;
    let binary_expression = unwrap_or_return_none!(body.as_r_binary_expression());
    let operator = binary_expression.operator()?;
    let operator_kind = operator.kind();
    let left = binary_expression.left()?;
    let right = binary_expression.right()?;

    if operator_kind != RSyntaxKind::GREATER_THAN
        && operator_kind != RSyntaxKind::GREATER_THAN_OR_EQUAL_TO
        && operator_kind != RSyntaxKind::LESS_THAN
        && operator_kind != RSyntaxKind::LESS_THAN_OR_EQUAL_TO
        && operator_kind != RSyntaxKind::EQUAL2
        && operator_kind != RSyntaxKind::NOT_EQUAL
    {
        return Ok(None);
    }

    let replacement_operator = match operator_kind {
        RSyntaxKind::GREATER_THAN => "<=",
        RSyntaxKind::GREATER_THAN_OR_EQUAL_TO => "<",
        RSyntaxKind::LESS_THAN => ">=",
        RSyntaxKind::LESS_THAN_OR_EQUAL_TO => ">",
        RSyntaxKind::EQUAL2 => "!=",
        RSyntaxKind::NOT_EQUAL => "==",
        // Safety: returned early if not one of the operators in this statement.
        _ => unreachable!(),
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            Rule::ComparisonNegation,
            format!("`!(x {} y)` can be simplified.", operator.text_trimmed()),
            Some(format!("Use `x {} y` instead.", replacement_operator)),
        ),
        range,
        Fix::new(
            range,
            format!(
                "{} {} {}",
                left.to_trimmed_text(),
                replacement_operator,
                right.to_trimmed_text()
            ),
            node_contains_comments(ast.syntax()),
        ),
    );

    Ok(Some(diagnostic))
}
