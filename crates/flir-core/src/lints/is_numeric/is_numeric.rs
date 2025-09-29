use crate::diagnostic::*;
use air_r_syntax::*;
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

pub fn is_numeric(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let operator = operator?;
    let left = left?;
    let right = right?;

    if operator.kind() != RSyntaxKind::OR2 {
        return Ok(None);
    };

    // Early returns: LHS or RHS are not functions
    let Some(left) = left.as_r_call() else {
        return Ok(None);
    };
    let Some(right) = right.as_r_call() else {
        return Ok(None);
    };

    let RCallFields { function: fun_left, arguments: arg_left } = left.as_fields();
    let fun_left = fun_left?;
    let arg_left = arg_left?;
    let RCallFields { function: fun_right, arguments: arg_right } = right.as_fields();
    let fun_right = fun_right?;
    let arg_right = arg_right?;

    // Early return: LHS or RHS are not the correct functions
    let left_is_numeric = fun_left.to_trimmed_text() == "is.numeric";
    let right_is_numeric = fun_right.to_trimmed_text() == "is.numeric";
    let left_is_integer = fun_left.to_trimmed_text() == "is.integer";
    let right_is_integer = fun_right.to_trimmed_text() == "is.integer";

    if !((left_is_integer && right_is_numeric) || (left_is_numeric && right_is_integer)) {
        return Ok(None);
    }

    // Early return: LHS and RHS args are not the same (e.g.
    // `is.numeric(x) || is.integer(y)`).
    let left_arg = arg_left.into_syntax().text_trimmed();
    let right_arg = arg_right.into_syntax().text_trimmed();
    if left_arg != right_arg {
        return Ok(None);
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        IsNumeric,
        range,
        Fix {
            content: format!("is.numeric{left_arg}"),
            start: range.start().into(),
            end: range.end().into(),
        },
    );
    Ok(Some(diagnostic))
}
