use crate::diagnostic::*;
use crate::utils::node_contains_comments;
use air_r_syntax::*;
use biome_rowan::AstNode;

/// Version added: 0.0.8
///
/// ## What it does
///
/// Check for `x == NA`, `x != NA`, `x %in% NA` and `x %notin% NA`, and
/// replaces those by `is.na()` or `!is.na()` calls.
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
pub fn equals_na(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let left = left?.to_trimmed_string();
    let operator = operator?;
    let right = right?.to_trimmed_string();

    let operator_is_in =
        operator.kind() == RSyntaxKind::SPECIAL && operator.text_trimmed() == "%in%";
    let operator_is_notin =
        operator.kind() == RSyntaxKind::SPECIAL && operator.text_trimmed() == "%notin%";

    if operator.kind() != RSyntaxKind::EQUAL2
        && operator.kind() != RSyntaxKind::NOT_EQUAL
        && !operator_is_in
        && !operator_is_notin
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

    let left_is_na = na_values.contains(&left.as_str());
    let right_is_na = na_values.contains(&right.as_str());

    // `NA %in% x` is equivalent to `anyNA(x)`, not `is.na(x)`
    // `NA %notin% x` is equivalent to `!anyNA(x)`, not `!is.na(x)`
    if (operator_is_in || operator_is_notin) && left_is_na {
        return Ok(None);
    }

    // If NA is quoted in text, then quotation marks are escaped and this
    // is false.
    if (left_is_na && right_is_na) || (!left_is_na && !right_is_na) {
        return Ok(None);
    }
    let range = ast.syntax().text_trimmed_range();

    let replacement = if left_is_na { right } else { left };

    let operator_text = operator.text_trimmed();
    let should_negate = match operator.kind() {
        RSyntaxKind::EQUAL2 => false,
        RSyntaxKind::NOT_EQUAL => true,
        RSyntaxKind::SPECIAL if operator_text == "%in%" => false,
        RSyntaxKind::SPECIAL if operator_text == "%notin%" => true,
        _ => unreachable!("This case is an early return"),
    };

    let replacement = if should_negate {
        format!("!is.na({replacement})")
    } else {
        format!("is.na({replacement})")
    };
    let suggestion = if should_negate {
        "Use `!is.na()` instead."
    } else {
        "Use `is.na()` instead."
    };

    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "equals_na".to_string(),
            format!("Comparing to NA with `{operator_text}` is problematic."),
            Some(suggestion.to_string()),
        ),
        range,
        Fix {
            content: replacement,
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
