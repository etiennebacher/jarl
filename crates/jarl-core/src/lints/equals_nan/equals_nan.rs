use crate::diagnostic::*;
use crate::utils::node_contains_comments;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct EqualsNaN;

/// ## What it does
///
/// Check for `x == NaN`, `x != NaN` and `x %in% NaN`, and replaces those by
/// `is.nan()` calls.
///
/// ## Why is this bad?
///
/// Comparing a value to `NaN` using `==` returns `NaN` in many cases:
/// ```r
/// x <- c(1, 2, 3, NaN)
/// x == NaN
/// #> [1] NA NA NA NA
/// ```
/// which is very likely not the expected output.
///
/// ## Example
///
/// ```r
/// x <- c(1, 2, 3, NaN)
/// x == NaN
/// ```
///
/// Use instead:
/// ```r
/// x <- c(1, 2, 3, NaN)
/// is.nan(x)
/// ```
impl Violation for EqualsNaN {
    fn name(&self) -> String {
        "equals_nan".to_string()
    }
    fn body(&self) -> String {
        "Comparing to NaN with `==`, `!=` or `%in%` is problematic.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `is.nan()` instead.".to_string())
    }
}

pub fn equals_nan(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let left = left?;
    let operator = operator?;
    let right = right?;

    let operator_is_in =
        operator.kind() == RSyntaxKind::SPECIAL && operator.text_trimmed() == "%in%";

    if operator.kind() != RSyntaxKind::EQUAL2
        && operator.kind() != RSyntaxKind::NOT_EQUAL
        && !operator_is_in
    {
        return Ok(None);
    };

    let left_is_nan = left.as_r_nan_expression().is_some();
    let right_is_nan = right.as_r_nan_expression().is_some();

    // `x %in% NaN` returns missings, but `NaN %in% x` returns TRUE/FALSE.
    if operator_is_in && left_is_nan {
        return Ok(None);
    }

    // If NA is quoted in text, then quotation marks are escaped and this
    // is false.
    if (left_is_nan && right_is_nan) || (!left_is_nan && !right_is_nan) {
        return Ok(None);
    }
    let range = ast.syntax().text_trimmed_range();

    let replacement = if left_is_nan {
        right.to_trimmed_string()
    } else {
        left.to_trimmed_string()
    };

    let diagnostic = match operator.kind() {
        RSyntaxKind::EQUAL2 => Diagnostic::new(
            EqualsNaN,
            range,
            Fix {
                content: format!("is.nan({replacement})"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        ),
        RSyntaxKind::NOT_EQUAL => Diagnostic::new(
            EqualsNaN,
            range,
            Fix {
                content: format!("!is.nan({replacement})"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        ),
        RSyntaxKind::SPECIAL if operator.text_trimmed() == "%in%" => Diagnostic::new(
            EqualsNaN,
            range,
            Fix {
                content: format!("is.nan({replacement})"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        ),
        _ => unreachable!("This case is an early return"),
    };

    Ok(Some(diagnostic))
}
