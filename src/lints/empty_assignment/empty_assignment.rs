use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;
use anyhow::Result;
use biome_rowan::AstNode;

pub struct EmptyAssignment;

impl Violation for EmptyAssignment {
    fn name(&self) -> String {
        "empty_assignment".to_string()
    }
    fn body(&self) -> String {
        "Assign NULL explicitly or, whenever possible, allocate the empty object`.".to_string()
    }
}

impl LintChecker for EmptyAssignment {
    fn check(&self, ast: &RSyntaxNode, file: &str) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        let bin_expr = RBinaryExpression::cast(ast.clone());

        if bin_expr.is_none() {
            return Ok(diagnostics);
        }

        let RBinaryExpressionFields { left, operator, right } = bin_expr.unwrap().as_fields();

        let left = left?;
        let right = right?;
        let operator = operator?;

        if operator.kind() != RSyntaxKind::EQUAL
            && operator.kind() != RSyntaxKind::ASSIGN
            && operator.kind() != RSyntaxKind::ASSIGN_RIGHT
        {
            return Ok(diagnostics);
        };

        let value_is_empty = match operator.kind() {
            RSyntaxKind::EQUAL | RSyntaxKind::ASSIGN => {
                if let Some(right) = RBracedExpressions::cast(right.into()) {
                    right.expressions().text() == ""
                } else {
                    return Ok(diagnostics);
                }
            }
            RSyntaxKind::ASSIGN_RIGHT => {
                if let Some(left) = RBracedExpressions::cast(left.into()) {
                    left.expressions().text() == ""
                } else {
                    return Ok(diagnostics);
                }
            }
            _ => unreachable!("cannot have something else than an assignment"),
        };

        if value_is_empty {
            let range = ast.text_trimmed_range();
            diagnostics.push(Diagnostic::new(
                EmptyAssignment,
                file,
                range,
                Fix::empty(),
            ));
        }

        Ok(diagnostics)
    }
}
