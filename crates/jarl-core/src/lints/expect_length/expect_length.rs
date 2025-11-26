use crate::diagnostic::*;
use crate::utils::{get_arg_by_name_then_position, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

/// ## What it does
///
/// Checks for usage of `expect_equal(length(x), n)` and `expect_identical(length(x), n)`.
///
/// ## Why is this bad?
///
/// `expect_length(x, n)` is more explicit and clearer in intent than using
/// `expect_equal()` or `expect_identical()` with `length()`. It also provides
/// better error messages when tests fail.
///
/// This rule is **disabled by default**. Select it either with the rule name
/// `"expect_length"` or with the rule group `"TESTTHAT"`.
///
/// ## Example
///
/// ```r
/// expect_equal(length(x), 2)
/// expect_identical(length(x), n)
/// expect_equal(2L, length(x))
/// ```
///
/// Use instead:
/// ```r
/// expect_length(x, 2)
/// expect_length(x, n)
/// expect_length(x, 2L)
/// ```
pub fn expect_length(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let function_name = function.to_trimmed_text();

    // Only check expect_equal and expect_identical
    if function_name != "expect_equal" && function_name != "expect_identical" {
        return Ok(None);
    }

    let args = ast.arguments()?.items();

    let object = unwrap_or_return_none!(get_arg_by_name_then_position(&args, "object", 1));
    let expected = unwrap_or_return_none!(get_arg_by_name_then_position(&args, "expected", 2));

    let object_value = unwrap_or_return_none!(object.value());
    let expected_value = unwrap_or_return_none!(expected.value());

    // expect_length() doesn't support info=, label=, or expected.label= arguments
    if args.iter().count() > 2 {
        return Ok(None);
    }

    // Check for two patterns:
    // 1. expect_equal(length(x), n)
    // 2. expect_equal(n, length(x))

    let (length_arg, other_arg) =
    // Check pattern 1
    if let Some(object_call) = object_value.as_r_call() {
        let obj_fn = object_call.function()?;

        // If we're here, the first object is a call, with two options:
        // - the call is `length(...)`, great.
        // - the call is `foo(...)` and we have to check the value of `expected`
        //   because it could be that the general call is
        //   `expect_equal(foo(x), length(y))`, which we want to report.
        if obj_fn.to_trimmed_text() == "length" {
            (object_call, expected_value)
        } else if let Some(expected_call) = expected_value.as_r_call() {
            let exp_fn = expected_call.function()?;
            if exp_fn.to_trimmed_text() == "length" {
                (expected_call, object_value)
            } else {
                return Ok(None);
            }
        } else {
            return Ok(None);
        }
    } else if let Some(expected_call) = expected_value.as_r_call() {

        // If we're here, it means that the `object` isn't `length(...)`, so if
        // `expected` also isn't `length(...)` we stop.
        let exp_fn = expected_call.function()?;
        if exp_fn.to_trimmed_text() == "length" {
            (expected_call, object_value)
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    };

    // Don't lint if the other argument is also a length() call
    // e.g., expect_equal(length(x), length(y))
    if let Some(other_call) = other_arg.as_r_call() {
        let other_fn = other_call.function()?;
        if other_fn.to_trimmed_text() == "length" {
            return Ok(None);
        }
    }

    // Extract the argument to length()
    let length_x_arg = unwrap_or_return_none!(get_arg_by_name_then_position(
        &length_arg.arguments()?.items(),
        "x",
        1
    ));
    let length_x_value = unwrap_or_return_none!(length_x_arg.value());

    let x_text = length_x_value.to_trimmed_text();
    let n_text = other_arg.to_trimmed_text();

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "expect_length".to_string(),
            format!(
                "`expect_length(x, n)` is better than `{}(length(x), n)`.",
                function_name
            ),
            Some("Use `expect_length(x, n)` instead.".to_string()),
        ),
        range,
        Fix {
            content: format!("expect_length({}, {})", x_text, n_text),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
