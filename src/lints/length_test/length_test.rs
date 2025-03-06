use crate::location::Location;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::utils::find_row_col;
use air_r_syntax::RSyntaxKind::*;
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;
use anyhow::{Context, Result};
use biome_rowan::AstNode;

pub struct LengthTest;

impl Violation for LengthTest {
    fn name(&self) -> String {
        "length_test".to_string()
    }
    fn body(&self) -> String {
        "Checking the length of a logical vector is likely a mistake".to_string()
    }
}

impl LintChecker for LengthTest {
    fn check(
        &self,
        ast: &RSyntaxNode,
        loc_new_lines: &[usize],
        file: &str,
    ) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        let call = RCall::cast(ast.clone());
        if call.is_none() {
            return Ok(diagnostics);
        }
        let RCallFields { function, arguments } = call.unwrap().as_fields();
        let function = function?;

        if function.text() != "length" {
            return Ok(diagnostics);
        }

        let arguments = arguments?.items();
        let mut arg_is_binary_expr = false;
        let mut operator_text: String = "".to_string();
        let mut lhs: String = "".to_string();
        let mut rhs: String = "".to_string();

        if let Some(first_arg) = arguments.into_iter().nth(0) {
            if let Ok(x) = first_arg {
                let RArgumentFields { name_clause: _, value } = x.as_fields();
                let value = value.context("Found named argument without any value")?;
                if let AnyRExpression::RBinaryExpression(y) = value {
                    let RBinaryExpressionFields { left, operator, right } = y.as_fields();

                    let operator = operator?;
                    arg_is_binary_expr = operator.kind() == EQUAL2
                        || operator.kind() == GREATER_THAN
                        || operator.kind() == GREATER_THAN_OR_EQUAL_TO
                        || operator.kind() == LESS_THAN
                        || operator.kind() == LESS_THAN_OR_EQUAL_TO
                        || operator.kind() == NOT_EQUAL;

                    operator_text.push_str(operator.text_trimmed());
                    lhs.push_str(&left?.text());
                    rhs.push_str(&right?.text());
                }
            }
        } else {
            return Ok(diagnostics);
        }

        if arg_is_binary_expr {
            let (row, column) = find_row_col(ast, loc_new_lines);
            let range = ast.text_trimmed_range();
            diagnostics.push(Diagnostic {
                message: LengthTest.into(),
                filename: file.into(),
                location: Location { row, column },
                fix: Fix {
                    content: format!("length({}) {} {}", lhs, operator_text, rhs),
                    start: range.start().into(),
                    end: range.end().into(),
                },
            });
        }

        Ok(diagnostics)
    }
}
