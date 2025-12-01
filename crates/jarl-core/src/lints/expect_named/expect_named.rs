use crate::diagnostic::*;
use crate::utils::{
    get_arg_by_name_then_position, get_function_name, get_function_namespace_prefix,
    node_contains_comments,
};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for usage of `expect_equal(names(x), n)` and `expect_identical(names(x), n)`.
///
/// ## Why is this bad?
///
/// `expect_named(x, n)` is more explicit and clearer in intent than using
/// `expect_equal()` or `expect_identical()` with `names()`. It also provides
/// better error messages when tests fail.
///
/// This rule is **disabled by default**. Select it either with the rule name
/// `"expect_named"` or with the rule group `"TESTTHAT"`.
///
/// ## Example
///
/// ```r
/// expect_equal(names(x), "a")
/// expect_identical(names(x), c("a", "b"))
/// ```
///
/// Use instead:
/// ```r
/// expect_named(x, "a")
/// expect_named(x, c("a", "b"))
/// ```
pub fn expect_named(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let function_name = get_function_name(function.clone());

    // Only check expect_equal and expect_identical
    if function_name != "expect_equal" && function_name != "expect_identical" {
        return Ok(None);
    }

    let args = ast.arguments()?.items();

    let object = unwrap_or_return_none!(get_arg_by_name_then_position(&args, "object", 1));
    let expected = unwrap_or_return_none!(get_arg_by_name_then_position(&args, "expected", 2));

    let object_value = unwrap_or_return_none!(object.value());
    let expected_value = unwrap_or_return_none!(expected.value());

    // Only check for pattern: expect_equal(names(x), n)

    let (names_arg, other_arg) = if let Some(object_call) = object_value.as_r_call() {
        let obj_fn = object_call.function()?;
        let obj_fn_name = get_function_name(obj_fn);

        // Only report if names() is in the object (first) argument
        if obj_fn_name == "names" {
            (object_call, expected_value)
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    };

    // Don't lint if the other argument is also a names() call
    // e.g., expect_equal(names(x), names(y))
    if let Some(other_call) = other_arg.as_r_call() {
        let other_fn = other_call.function()?;
        let other_fn_name = get_function_name(other_fn);

        if other_fn_name == "names" {
            return Ok(None);
        }
    }

    // Extract the argument to names()
    let names_args = names_arg.arguments()?.items();
    let names_x_arg = unwrap_or_return_none!(get_arg_by_name_then_position(&names_args, "x", 1));
    let names_x_value = unwrap_or_return_none!(names_x_arg.value());

    let x_text = names_x_value.to_trimmed_text();
    let n_text = other_arg.to_trimmed_text();

    // Preserve namespace prefix if present
    let namespace_prefix = get_function_namespace_prefix(function).unwrap_or_default();

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "expect_named".to_string(),
            format!(
                "`expect_named(x, n)` is better than `{}(names(x), n)`.",
                function_name
            ),
            Some("Use `expect_named(x, n)` instead.".to_string()),
        ),
        range,
        Fix {
            content: format!("{}expect_named({}, {})", namespace_prefix, x_text, n_text),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
