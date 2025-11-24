use crate::diagnostic::*;
use crate::utils::{get_arg_by_name_then_position, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for usage of `expect_equal(x, TRUE)`, `expect_equal(x, FALSE)`,
/// `expect_identical(x, TRUE)`, and `expect_identical(x, FALSE)` in tests.
///
/// ## Why is this bad?
///
/// `expect_true()` and `expect_false()` are more explicit and clearer in intent
/// than comparing with `expect_equal()` or `expect_identical()`. They also
/// provide better error messages when tests fail.
///
/// This rule is **disabled by default**. Select it either with the rule name
/// `"expect_true_false"` or with the rule group `"TESTTHAT"`.
///
/// ## Example
///
/// ```r
/// expect_equal(is.numeric(x), TRUE)
/// expect_identical(is.character(y), FALSE)
/// ```
///
/// Use instead:
/// ```r
/// expect_true(is.numeric(x))
/// expect_false(is.character(y))
/// ```
pub fn expect_true_false(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let function_name = function.to_trimmed_text();

    // Check if this is expect_equal or expect_identical
    if function_name != "expect_equal" && function_name != "expect_identical" {
        return Ok(None);
    }

    let args = ast.arguments()?.items();

    // Get `object` and `expected` arguments
    let object = get_arg_by_name_then_position(&args, "object", 1);
    let expected = get_arg_by_name_then_position(&args, "expected", 2);

    if object.is_none() || expected.is_none() {
        return Ok(None);
    }

    let object_value = object.unwrap().value();
    let expected_value = expected.unwrap().value();

    if object_value.is_none() || expected_value.is_none() {
        return Ok(None);
    }

    let object_text = object_value.unwrap().to_trimmed_text();
    let expected_text = expected_value.unwrap().to_trimmed_text();

    // Check if either argument is TRUE or FALSE (but not a vector like c(TRUE, FALSE))
    let object_is_true = object_text == "TRUE";
    let object_is_false = object_text == "FALSE";
    let expected_is_true = expected_text == "TRUE";
    let expected_is_false = expected_text == "FALSE";

    let (is_true, other_arg_text) = if object_is_true {
        (true, expected_text)
    } else if object_is_false {
        (false, expected_text)
    } else if expected_is_true {
        (true, object_text)
    } else if expected_is_false {
        (false, object_text)
    } else {
        return Ok(None);
    };

    let range = ast.syntax().text_trimmed_range();
    let (new_function, msg, suggestion) = if is_true {
        (
            "expect_true",
            format!(
                "`{}(x, TRUE)` is not as clear as `expect_true(x)`.",
                function_name
            ),
            "Use `expect_true(x)` instead.",
        )
    } else {
        (
            "expect_false",
            format!(
                "`{}(x, FALSE)` is not as clear as `expect_false(x)`.",
                function_name
            ),
            "Use `expect_false(x)` instead.",
        )
    };

    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "expect_true_false".to_string(),
            msg,
            Some(suggestion.to_string()),
        ),
        range,
        Fix {
            content: format!("{}({})", new_function, other_arg_text),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
