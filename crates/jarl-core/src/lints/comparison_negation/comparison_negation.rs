use crate::diagnostic::*;
use crate::utils::node_contains_comments;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub fn comparison_negation(ast: &RUnaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let operator = ast.operator()?;

    if operator.kind() != RSyntaxKind::BANG {
        return Ok(None);
    }

    let argument = ast.argument()?;
    let argument = argument.as_r_parenthesized_expression();
    if argument.is_none() {
        return Ok(None);
    }
    // Safety: can unwrap() here, we returned early if it's None.
    let binary_expression = argument.unwrap();
    let binary_expression = binary_expression.body()?;
    let binary_expression = binary_expression.as_r_binary_expression();
    if binary_expression.is_none() {
        return Ok(None);
    }
    // Safety: can unwrap() here, we returned early if it's None.
    let binary_expression = binary_expression.unwrap();
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
            "comparison_negation".to_string(),
            format!("Do not use `!(x {} y)`.", operator.text_trimmed()),
            Some(format!("Use `x {} y` instead.", replacement_operator)),
        ),
        range,
        Fix {
            content: format!(
                "{} {} {}",
                left.to_trimmed_text(),
                replacement_operator,
                right.to_trimmed_text()
            ),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
