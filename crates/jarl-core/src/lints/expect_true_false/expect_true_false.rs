use crate::diagnostic::*;
use crate::utils::{get_arg_by_position, node_contains_comments};
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

    // Get first and second arguments
    let first_arg = get_arg_by_position(&args, 1);
    let second_arg = get_arg_by_position(&args, 2);

    if first_arg.is_none() || second_arg.is_none() {
        return Ok(None);
    }

    let first_arg = first_arg.unwrap();
    let second_arg = second_arg.unwrap();

    let first_value = first_arg.value();
    let second_value = second_arg.value();

    if first_value.is_none() || second_value.is_none() {
        return Ok(None);
    }

    let first_text = first_value.unwrap().to_trimmed_text();
    let second_text = second_value.unwrap().to_trimmed_text();

    // Check if either argument is TRUE or FALSE (but not a vector like c(TRUE, FALSE))
    let first_is_true = first_text == "TRUE";
    let first_is_false = first_text == "FALSE";
    let second_is_true = second_text == "TRUE";
    let second_is_false = second_text == "FALSE";

    // Skip if this is a vector comparison (e.g., c(TRUE, FALSE))
    if first_text.starts_with("c(") || second_text.starts_with("c(") {
        return Ok(None);
    }

    let (is_true, other_arg_text) = if first_is_true {
        (true, second_text)
    } else if first_is_false {
        (false, second_text)
    } else if second_is_true {
        (true, first_text)
    } else if second_is_false {
        (false, first_text)
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
