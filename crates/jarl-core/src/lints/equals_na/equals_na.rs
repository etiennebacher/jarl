use crate::diagnostic::*;
use crate::utils::node_contains_comments;
use air_r_syntax::*;
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
/// #> [1] NA NA NA NA
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
        "Comparing to NA with `==`, `!=` or `%in%` is problematic.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `is.na()` instead.".to_string())
    }
}

pub fn equals_na(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let left = left?.to_trimmed_string();
    let operator = operator?;
    let right = right?.to_trimmed_string();

    let operator_is_in =
        operator.kind() == RSyntaxKind::SPECIAL && operator.text_trimmed() == "%in%";

    if operator.kind() != RSyntaxKind::EQUAL2
        && operator.kind() != RSyntaxKind::NOT_EQUAL
        && !operator_is_in
    {
        return Ok(None);
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

    // `x %in% NA` is equivalent to `anyNA(x)`, not `is.na(x)`
    if operator_is_in && left_is_na {
        return Ok(None);
    }

    // If NA is quoted in text, then quotation marks are escaped and this
    // is false.
    if (left_is_na && right_is_na) || (!left_is_na && !right_is_na) {
        return Ok(None);
    }
    let range = ast.syntax().text_trimmed_range();

    let replacement = if left_is_na {
        right.trim().to_string()
    } else {
        left.trim().to_string()
    };

    let diagnostic = match operator.kind() {
        RSyntaxKind::EQUAL2 => Diagnostic::new(
            EqualsNa,
            range,
            Fix {
                content: format!("is.na({replacement})"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        ),
        RSyntaxKind::NOT_EQUAL => Diagnostic::new(
            EqualsNa,
            range,
            Fix {
                content: format!("!is.na({replacement})"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        ),
        RSyntaxKind::SPECIAL if operator.text_trimmed() == "%in%" => Diagnostic::new(
            EqualsNa,
            range,
            Fix {
                content: format!("is.na({replacement})"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        ),
        _ => unreachable!("This case is an early return"),
    };

    Ok(Some(diagnostic))
}
