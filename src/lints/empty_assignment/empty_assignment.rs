use crate::message::*;
use crate::trait_lint_checker::LintChecker;
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

impl LintChecker for EmptyAssignment {
    fn check(&self, ast: &AnyRExpression) -> Result<Diagnostic> {
        let mut diagnostic = Diagnostic::empty();
        let ast = if let Some(ast) = ast.as_r_binary_expression() {
            ast
        } else {
            return Ok(diagnostic);
        };

        let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

        let left = left?;
        let right = right?;
        let operator = operator?;

        if operator.kind() != RSyntaxKind::EQUAL
            && operator.kind() != RSyntaxKind::ASSIGN
            && operator.kind() != RSyntaxKind::ASSIGN_RIGHT
        {
            return Ok(diagnostic);
        };

        let value_is_empty = match operator.kind() {
            RSyntaxKind::EQUAL | RSyntaxKind::ASSIGN => {
                match RBracedExpressions::cast(right.into()) {
                    Some(right) => right.expressions().text() == "",
                    _ => {
                        return Ok(diagnostic);
                    }
                }
            }
            RSyntaxKind::ASSIGN_RIGHT => match RBracedExpressions::cast(left.into()) {
                Some(left) => left.expressions().text() == "",
                _ => {
                    return Ok(diagnostic);
                }
            },
            _ => unreachable!("cannot have something else than an assignment"),
        };

        if value_is_empty {
            let range = ast.clone().into_syntax().text_trimmed_range();
            diagnostic = Diagnostic::new(EmptyAssignment, range, Fix::empty());
        }

        Ok(diagnostic)
    }
}
