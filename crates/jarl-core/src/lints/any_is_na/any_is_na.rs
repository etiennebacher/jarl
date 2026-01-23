use crate::diagnostic::*;
use crate::utils::{get_nested_functions_content, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for usage of `any(is.na(...))` and `NA %in% x`.
///
/// ## Why is this bad?
///
/// While both cases are valid R code, the base R function `anyNA()` is more
/// efficient (both in speed and memory used).
///
/// ## Example
///
/// ```r
/// x <- c(1:10000, NA)
/// any(is.na(x))
/// NA %in% x
/// ```
///
/// Use instead:
/// ```r
/// x <- c(1:10000, NA)
/// anyNA(x)
/// ```
///
/// ## References
///
/// See `?anyNA`
pub fn any_is_na(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let inner_content = get_nested_functions_content(ast, "any", "is.na")?;

    if let Some(inner_content) = inner_content {
        let range = ast.syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(
            ViolationData::new(
                "any_is_na".to_string(),
                "`any(is.na(...))` is inefficient.".to_string(),
                Some("Use `anyNA(...)` instead.".to_string()),
            ),
            range,
            Fix {
                content: format!("anyNA({inner_content})"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        );
        return Ok(Some(diagnostic));
    }

    Ok(None)
}

pub fn any_is_na_2(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let left = left?;
    let operator = operator?;
    let right = right?;

    let operator_is_in =
        operator.kind() == RSyntaxKind::SPECIAL && operator.text_trimmed() == "%in%";

    if !operator_is_in {
        return Ok(None);
    };

    let left_is_na = left.as_r_na_expression().is_some();
    let right_is_na = right.as_r_na_expression().is_some();

    // `x %in% NA` is not equivalent to anyNA(x)
    if operator_is_in && right_is_na {
        return Ok(None);
    }

    // If NA is quoted in text, then quotation marks are escaped and this
    // is false.
    if (left_is_na && right_is_na) || (!left_is_na && !right_is_na) {
        return Ok(None);
    }
    let range = ast.syntax().text_trimmed_range();

    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "any_is_na".to_string(),
            "`NA %in% x` is inefficient.".to_string(),
            Some("Use `anyNA(x)` instead.".to_string()),
        ),
        range,
        Fix {
            content: format!("anyNA({})", right.to_trimmed_string()),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
