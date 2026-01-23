use crate::diagnostic::*;
use crate::utils::node_contains_comments;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct EqualsNull;

/// ## What it does
///
/// Check for `x == NULL`, `x != NULL` and `x %in% NULL`, and replaces those by
/// `is.null()` calls.
///
/// ## Why is this bad?
///
/// Comparing a value to `NULL` using `==` returns a `logical(0)` in many cases:
/// ```r
/// x <- NULL
/// x == NULL
/// #> logical(0)
/// ```
/// which is very likely not the expected output.
///
/// ## Example
///
/// ```r
/// x <- c(1, 2, 3)
/// y <- NULL
///
/// x == NULL
/// y == NULL
/// ```
///
/// Use instead:
/// ```r
/// x <- c(1, 2, 3)
/// y <- NULL
///
/// is.null(x)
/// is.null(y)
/// ```
impl Violation for EqualsNull {
    fn name(&self) -> String {
        "equals_null".to_string()
    }
    fn body(&self) -> String {
        "Comparing to NULL with `==`, `!=` or `%in%` is problematic.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `is.null()` instead.".to_string())
    }
}

pub fn equals_null(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let left = left?;
    let operator = operator?;
    let right = right?;

    if operator.kind() != RSyntaxKind::EQUAL2
        && operator.kind() != RSyntaxKind::NOT_EQUAL
        && (operator.kind() != RSyntaxKind::SPECIAL || operator.text_trimmed() != "%in%")
    {
        return Ok(None);
    };

    let left_is_null = left.as_r_null_expression().is_some();
    let right_is_null = right.as_r_null_expression().is_some();

    if (left_is_null && right_is_null) || (!left_is_null && !right_is_null) {
        return Ok(None);
    }
    let range = ast.syntax().text_trimmed_range();

    let replacement = if left_is_null {
        right.to_trimmed_string()
    } else {
        left.to_trimmed_string()
    };

    let diagnostic = match operator.kind() {
        RSyntaxKind::EQUAL2 => Diagnostic::new(
            EqualsNull,
            range,
            Fix {
                content: format!("is.null({replacement})"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        ),
        RSyntaxKind::NOT_EQUAL => Diagnostic::new(
            EqualsNull,
            range,
            Fix {
                content: format!("!is.null({replacement})"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        ),
        RSyntaxKind::SPECIAL if operator.text_trimmed() == "%in%" => Diagnostic::new(
            EqualsNull,
            range,
            Fix {
                content: format!("is.null({replacement})"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        ),
        _ => unreachable!("This case is an early return"),
    };

    Ok(Some(diagnostic))
}
