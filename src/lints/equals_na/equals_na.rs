use crate::location::Location;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::utils::find_row_col;
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct EqualsNa;

impl Violation for EqualsNa {
    fn name(&self) -> String {
        "equals_na".to_string()
    }
    fn body(&self) -> String {
        "Use `is.na()` instead of comparing to NA with ==, != or %in%.".to_string()
    }
}

impl LintChecker for EqualsNa {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &[usize], file: &str) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        let bin_expr = RBinaryExpression::cast(ast.clone());
        if bin_expr.is_none() {
            return diagnostics;
        }

        let RBinaryExpressionFields { left, operator, right } = bin_expr.unwrap().as_fields();

        let left = left.unwrap();
        let operator = operator.unwrap();
        let right = right.unwrap();

        if operator.kind() != RSyntaxKind::EQUAL2 && operator.kind() != RSyntaxKind::NOT_EQUAL {
            return diagnostics;
        };

        let na_values = [
            "NA",
            "NA_character_",
            "NA_integer_",
            "NA_real_",
            "NA_logical_",
            "NA_complex_",
        ];

        let left_is_na = na_values.contains(&left.to_string().trim());
        let right_is_na = na_values.contains(&right.to_string().trim());

        // If NA is quoted in text, then quotation marks are escaped and this
        // is false.
        if (left_is_na && right_is_na) || (!left_is_na && !right_is_na) {
            return diagnostics;
        }
        let (row, column) = find_row_col(ast, loc_new_lines);
        let range = ast.text_trimmed_range();

        let replacement = if left_is_na {
            right.to_string().trim().to_string()
        } else {
            left.to_string().trim().to_string()
        };

        match operator.kind() {
            RSyntaxKind::EQUAL2 => {
                diagnostics.push(Diagnostic {
                    message: EqualsNa.into(),
                    filename: file.into(),
                    location: Location { row, column },
                    fix: Fix {
                        content: format!("is.na({})", replacement),
                        start: range.start().into(),
                        end: range.end().into(),
                    },
                });
            }
            RSyntaxKind::NOT_EQUAL => {
                diagnostics.push(Diagnostic {
                    message: EqualsNa.into(),
                    filename: file.into(),
                    location: Location { row, column },
                    fix: Fix {
                        content: format!("!is.na({})", replacement),
                        start: range.start().into(),
                        end: range.end().into(),
                    },
                });
            }
            _ => unreachable!("This case is an early return"),
        };

        diagnostics
    }
}
