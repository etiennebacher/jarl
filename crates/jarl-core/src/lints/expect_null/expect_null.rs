use crate::diagnostic::*;
use crate::utils::{get_arg_by_name_then_position, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for usage of `expect_equal(x, NULL)`, `expect_identical(x, NULL)`,
/// and `expect_true(is.null(x))`.
///
/// ## Why is this bad
///
/// `expect_null()` is more explicit and clearer in intent than comparing with
/// `expect_equal()`, `expect_identical()`, or wrapping `is.null()` in
/// `expect_true()`. It also provides better error messages when tests fail.
///
/// This rule is **disabled by default**. Select it either with the rule name
/// `"expect_null"` or with the rule group `"TESTTHAT"`.
///
/// ## Example
///
/// ```r
/// expect_equal(x, NULL)
/// expect_identical(x, NULL)
/// expect_true(is.null(foo(x)))
/// ```
///
/// Use instead:
/// ```r
/// expect_null(x)
/// expect_null(x)
/// expect_null(foo(x))
/// ```
pub fn expect_null(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let function_name = function.to_trimmed_text();

    // Case 1: expect_equal(x, NULL) or expect_identical(x, NULL)
    if function_name == "expect_equal" || function_name == "expect_identical" {
        return check_expect_equal_null(ast, &function_name);
    }

    // Case 2: expect_true(is.null(x))
    if function_name == "expect_true" {
        return check_expect_true_is_null(ast);
    }

    Ok(None)
}

fn check_expect_equal_null(ast: &RCall, function_name: &str) -> anyhow::Result<Option<Diagnostic>> {
    let args = ast.arguments()?.items();

    let Some(object) = get_arg_by_name_then_position(&args, "object", 1) else {
        return Ok(None);
    };
    let Some(expected) = get_arg_by_name_then_position(&args, "expected", 2) else {
        return Ok(None);
    };

    let Some(object_value) = object.value() else {
        return Ok(None);
    };
    let Some(expected_value) = expected.value() else {
        return Ok(None);
    };

    let object_kind = object_value.syntax().kind();
    let expected_kind = expected_value.syntax().kind();

    let other_arg_text = if expected_kind == RSyntaxKind::R_NULL_EXPRESSION {
        object_value.to_trimmed_text()
    } else if object_kind == RSyntaxKind::R_NULL_EXPRESSION {
        expected_value.to_trimmed_text()
    } else {
        return Ok(None);
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "expect_null".to_string(),
            format!(
                "`{}(x, NULL)` is not as clear as `expect_null(x)`.",
                function_name
            ),
            Some("Use `expect_null(x)` instead.".to_string()),
        ),
        range,
        Fix {
            content: format!("expect_null({})", other_arg_text),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}

fn check_expect_true_is_null(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let args = ast.arguments()?.items();

    let Some(object) = get_arg_by_name_then_position(&args, "object", 1) else {
        return Ok(None);
    };

    let Some(object_value) = object.value() else {
        return Ok(None);
    };

    // Check if it's a call to `is.null()`
    let Some(call) = object_value.as_r_call() else {
        return Ok(None);
    };
    let function = call.function()?;
    let function_name = function.to_trimmed_text();
    if function_name != "is.null" {
        return Ok(None);
    }

    // Get the argument to `is.null()`
    let inner_args = call.arguments()?.items();
    let Some(inner_arg) = get_arg_by_name_then_position(&inner_args, "x", 1) else {
        return Ok(None);
    };

    let Some(inner_value) = inner_arg.value() else {
        return Ok(None);
    };

    let inner_text = inner_value.to_trimmed_text();

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "expect_null".to_string(),
            "`expect_true(is.null(x))` is not as clear as `expect_null(x)`.".to_string(),
            Some("Use `expect_null(x)` instead.".to_string()),
        ),
        range,
        Fix {
            content: format!("expect_null({})", inner_text),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
