use crate::location::Location;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::utils::find_row_col;
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct RedundantEquals;

impl Violation for RedundantEquals {
    fn name(&self) -> String {
        "redundant_equals".to_string()
    }
    fn body(&self) -> String {
        "Using == on a logical vector is redundant.".to_string()
    }
}

impl LintChecker for RedundantEquals {
    fn check(
        &self,
        ast: &RSyntaxNode,
        loc_new_lines: &[usize],
        file: &str,
    ) -> anyhow::Result<Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        let bin_expr = RBinaryExpression::cast(ast.clone());

        if bin_expr.is_none() {
            return Ok(diagnostics);
        }

        let RBinaryExpressionFields { left, operator, right } = bin_expr.unwrap().as_fields();

        let operator = operator?;
        let left = left?;
        let right = right?;

        let left_is_true = &left.as_r_true_expression().is_some();
        let left_is_false = &left.as_r_false_expression().is_some();
        let right_is_true = &right.as_r_true_expression().is_some();
        let right_is_false = &right.as_r_false_expression().is_some();

        match operator.kind() {
            RSyntaxKind::EQUAL2 => {
                let fix = if *left_is_true {
                    right.text().to_string()
                } else if *right_is_true {
                    left.text().to_string()
                } else if *left_is_false {
                    format!("!{}", right.text())
                } else if *right_is_false {
                    format!("!{}", left.text())
                } else {
                    return Ok(diagnostics);
                };

                let (row, column) = find_row_col(ast, loc_new_lines);
                let range = ast.text_trimmed_range();
                diagnostics.push(Diagnostic {
                    message: RedundantEquals.into(),
                    filename: file.into(),
                    location: Location { row, column },
                    fix: Fix {
                        content: fix,
                        start: range.start().into(),
                        end: range.end().into(),
                    },
                });
            }
            RSyntaxKind::NOT_EQUAL => {
                let fix = if *left_is_true {
                    format!("!{}", right.text())
                } else if *right_is_true {
                    format!("!{}", left.text())
                } else if *left_is_false {
                    right.text().to_string()
                } else if *right_is_false {
                    left.text().to_string()
                } else {
                    return Ok(diagnostics);
                };
                let (row, column) = find_row_col(ast, loc_new_lines);
                let range = ast.text_trimmed_range();
                diagnostics.push(Diagnostic {
                    message: RedundantEquals.into(),
                    filename: file.into(),
                    location: Location { row, column },
                    fix: Fix {
                        content: fix,
                        start: range.start().into(),
                        end: range.end().into(),
                    },
                });
            }
            _ => return Ok(diagnostics),
        };
        Ok(diagnostics)
    }
}
