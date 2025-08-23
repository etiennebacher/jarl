use crate::message::*;
use air_r_syntax::*;
use anyhow::Result;
use biome_rowan::AstNode;

pub struct IsNumeric;

/// ## What it does
///
/// Checks for usage of `is.numeric(x) || is.integer(x)`.
///
/// ## Why is this bad?
///
/// `is.numeric(x)` returns `TRUE` when x is double or integer. Therefore,
/// testing `is.numeric(x) || is.integer(x)` is redundant and can be simplified.
///
/// ## Example
///
/// ```r
/// x <- 1:3
/// is.numeric(x) || is.integer(x)
/// ```
///
/// Use instead:
/// ```r
/// x <- 1:3
/// is.numeric(x)
/// ```
///
/// ## References
///
/// See `?is.numeric`
impl Violation for IsNumeric {
    fn name(&self) -> String {
        "is_numeric".to_string()
    }
    fn body(&self) -> String {
        "Use `is.numeric(x)` instead of the equivalent `is.numeric(x) || is.integer(x)`. `Use is.double(x)` to test for objects stored as 64-bit floating point.".to_string()
    }
}

pub fn is_numeric(ast: &RBinaryExpression) -> Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let operator = operator?;
    let left = left?;
    let right = right?;

    if operator.kind() != RSyntaxKind::OR2 {
        return Ok(None);
    };

    let lhs = left.clone().into_syntax();
    let rhs = right.clone().into_syntax();

    // Early return: LHS or RHS are not functions
    let left_is_function = lhs.kind() == RSyntaxKind::R_CALL;
    let right_is_function = rhs.kind() == RSyntaxKind::R_CALL;

    if !left_is_function || !right_is_function {
        return Ok(None);
    };

    // Early return: LHS or RHS are not the correct functions
    let left_is_numeric = lhs
        .first_child()
        .map(|x| x.text_trimmed() == "is.numeric")
        .unwrap_or(false);
    let right_is_numeric = rhs
        .first_child()
        .map(|x| x.text_trimmed() == "is.numeric")
        .unwrap_or(false);

    let left_is_integer = lhs
        .first_child()
        .map(|x| x.text_trimmed() == "is.integer")
        .unwrap_or(false);
    let right_is_integer = rhs
        .first_child()
        .map(|x| x.text_trimmed() == "is.integer")
        .unwrap_or(false);

    if !((left_is_integer && right_is_numeric) || (left_is_numeric && right_is_integer)) {
        return Ok(None);
    }

    // Early return: LHS and RHS args are not the same (e.g.
    // `is.numeric(x) || is.integer(y)`).
    let left_arg = left
        .as_r_call()
        .unwrap()
        .arguments()?
        .into_syntax()
        .text_trimmed();

    let right_arg = right
        .as_r_call()
        .unwrap()
        .arguments()?
        .into_syntax()
        .text_trimmed();

    if left_arg != right_arg {
        return Ok(None);
    };

    let range = ast.clone().into_syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        IsNumeric,
        range,
        Fix {
            content: format!("is.numeric{}", left_arg),
            start: range.start().into(),
            end: range.end().into(),
        },
    );
    Ok(Some(diagnostic))
}
