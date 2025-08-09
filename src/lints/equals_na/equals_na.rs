use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use air_r_syntax::*;
use anyhow::Result;
use biome_rowan::AstNode;

pub struct EqualsNa;

/// ## What it does
///
/// Check for `x == NA`, `x != NA` and `x %in% NA`, and replaces those by
/// `is.na()` calls.
///
/// ## Why is this bad?
///
/// Comparing a value to `NA` using `==` returns `NA` in many cases:
/// ```r
/// x <- c(1, 2, 3, NA)
/// x == NA
/// ```
/// which is very likely not the expected output.
///
/// ## Example
///
/// ```r
/// x <- c(1, 2, 3, NA)
/// x == NA
/// ```
///
/// Use instead:
/// ```r
/// x <- c(1, 2, 3, NA)
/// is.na(x)
/// ```
impl Violation for EqualsNa {
    fn name(&self) -> String {
        "equals_na".to_string()
    }
    fn body(&self) -> String {
        "Use `is.na()` instead of comparing to NA with ==, != or %in%.".to_string()
    }
}

impl LintChecker for EqualsNa {
    fn check(&self, ast: &AnyRExpression) -> Result<Diagnostic> {
        let mut diagnostic = Diagnostic::empty();
        let ast = if let Some(ast) = ast.as_r_binary_expression() {
            ast
        } else {
            return Ok(diagnostic);
        };

        let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

        let left = left?;
        let operator = operator?;
        let right = right?;

        if operator.kind() != RSyntaxKind::EQUAL2 && operator.kind() != RSyntaxKind::NOT_EQUAL {
            return Ok(diagnostic);
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
            return Ok(diagnostic);
        }
        let range = ast.clone().into_syntax().text_trimmed_range();

        let replacement = if left_is_na {
            right.to_string().trim().to_string()
        } else {
            left.to_string().trim().to_string()
        };

        match operator.kind() {
            RSyntaxKind::EQUAL2 => {
                diagnostic = Diagnostic::new(
                    EqualsNa,
                    range,
                    Fix {
                        content: format!("is.na({replacement})"),
                        start: range.start().into(),
                        end: range.end().into(),
                    },
                );
            }
            RSyntaxKind::NOT_EQUAL => {
                diagnostic = Diagnostic::new(
                    EqualsNa,
                    range,
                    Fix {
                        content: format!("!is.na({replacement})"),
                        start: range.start().into(),
                        end: range.end().into(),
                    },
                );
            }
            _ => unreachable!("This case is an early return"),
        };

        Ok(diagnostic)
    }
}
