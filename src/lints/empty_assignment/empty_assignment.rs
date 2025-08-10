use crate::message::*;
use air_r_syntax::*;
use anyhow::Result;
use biome_rowan::AstNode;

pub struct EmptyAssignment;

impl Violation for EmptyAssignment {
    fn name(&self) -> String {
        "empty_assignment".to_string()
    }
    fn body(&self) -> String {
        "Assign NULL explicitly or, whenever possible, allocate the empty object with the right type and size.".to_string()
    }
}

pub fn empty_assignment(ast: &RBinaryExpression) -> Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let left = left?;
    let right = right?;
    let operator = operator?;

    if operator.kind() != RSyntaxKind::EQUAL
        && operator.kind() != RSyntaxKind::ASSIGN
        && operator.kind() != RSyntaxKind::ASSIGN_RIGHT
    {
        return Ok(None);
    };

    let value_is_empty = match operator.kind() {
        RSyntaxKind::EQUAL | RSyntaxKind::ASSIGN => match RBracedExpressions::cast(right.into()) {
            Some(right) => right.expressions().text() == "",
            _ => {
                return Ok(None);
            }
        },
        RSyntaxKind::ASSIGN_RIGHT => match RBracedExpressions::cast(left.into()) {
            Some(left) => left.expressions().text() == "",
            _ => {
                return Ok(None);
            }
        },
        _ => unreachable!("cannot have something else than an assignment"),
    };

    if value_is_empty {
        let range = ast.clone().into_syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(EmptyAssignment, range, Fix::empty());
        return Ok(Some(diagnostic));
    }

    Ok(None)
}
