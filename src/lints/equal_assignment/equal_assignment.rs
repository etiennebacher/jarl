use crate::location::Location;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::utils::find_row_col;
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;
use anyhow::Result;
use biome_rowan::AstNode;

pub struct EqualAssignment;

impl Violation for EqualAssignment {
    fn name(&self) -> String {
        "equal_assignment".to_string()
    }
    fn body(&self) -> String {
        "Use <- for assignment.".to_string()
    }
}

impl LintChecker for EqualAssignment {
    fn check(
        &self,
        ast: &RSyntaxNode,
        loc_new_lines: &[usize],
        file: &str,
    ) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        let bin_expr = RBinaryExpression::cast(ast.clone());

        if bin_expr.is_none() {
            return Ok(diagnostics);
        }

        let RBinaryExpressionFields { left, operator, right } = bin_expr.unwrap().as_fields();

        let operator = operator?;
        let lhs = left?.into_syntax();
        let rhs = right?.into_syntax();

        if operator.kind() != RSyntaxKind::EQUAL && operator.kind() != RSyntaxKind::ASSIGN_RIGHT {
            return Ok(diagnostics);
        };

        let replacement = match operator.kind() {
            RSyntaxKind::EQUAL => {
                if lhs.kind() != RSyntaxKind::R_IDENTIFIER {
                    return Ok(diagnostics);
                }
                format!("{} <- {}", lhs.text_trimmed(), rhs.text_trimmed())
            }
            RSyntaxKind::ASSIGN_RIGHT => {
                if rhs.kind() != RSyntaxKind::R_IDENTIFIER {
                    return Ok(diagnostics);
                }
                format!("{} <- {}", rhs.text_trimmed(), lhs.text_trimmed())
            }
            _ => unreachable!(),
        };

        let (row, column) = find_row_col(ast, loc_new_lines);
        let range = ast.text_trimmed_range();
        diagnostics.push(Diagnostic {
            message: EqualAssignment.into(),
            filename: file.into(),
            location: Location { row, column },
            fix: Fix {
                content: replacement,
                start: range.start().into(),
                end: range.end().into(),
            },
        });

        Ok(diagnostics)
    }
}
