use crate::diagnostic::*;
use crate::utils::{
    get_arg_by_name_then_position, get_function_name, get_function_namespace_prefix,
    node_contains_comments,
};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

/// ## What it does
///
/// Checks for usage of `expect_equal(typeof(x), type)`,
/// `expect_identical(typeof(x), type)`, and `expect_true(is.<type>(x))` in tests.
///
/// ## Why is this bad?
///
/// `expect_type()` is more explicit and clearer in intent than comparing with
/// `expect_equal()`, `expect_identical()`, or wrapping type checks in
/// `expect_true()`. It also provides better error messages when tests fail.
///
/// This rule is **disabled by default**. Select it either with the rule name
/// `"expect_type"` or with the rule group `"TESTTHAT"`.
///
/// ## Example
///
/// ```r
/// expect_equal(typeof(x), "double")
/// expect_identical(typeof(x), "integer")
/// expect_true(is.character(x))
/// ```
///
/// Use instead:
/// ```r
/// expect_type(x, "double")
/// expect_type(x, "integer")
/// expect_type(x, "character")
/// ```
pub fn expect_type(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let function_name = get_function_name(function);

    // Case 1: expect_equal(typeof(x), type) or expect_identical(typeof(x), type)
    if function_name == "expect_equal" || function_name == "expect_identical" {
        return check_expect_equal_typeof(ast, &function_name);
    }

    // Case 2: expect_true(is.<type>(x))
    if function_name == "expect_true" {
        return check_expect_true_is_type(ast);
    }

    Ok(None)
}

fn check_expect_equal_typeof(
    ast: &RCall,
    function_name: &str,
) -> anyhow::Result<Option<Diagnostic>> {
    let args = ast.arguments()?.items();

    // expect_type() doesn't have info=, label=, or expected.label= arguments
    // If there are more than 2 arguments, skip the lint
    if args.iter().count() > 2 {
        return Ok(None);
    }

    let object = unwrap_or_return_none!(get_arg_by_name_then_position(&args, "object", 1));
    let expected = unwrap_or_return_none!(get_arg_by_name_then_position(&args, "expected", 2));

    let object_value = unwrap_or_return_none!(object.value());
    let expected_value = unwrap_or_return_none!(expected.value());

    // Check which argument is typeof(x) and which is the type
    let (typeof_call, type_text) = if is_typeof_call(&object_value) {
        (object_value, expected_value.to_trimmed_text())
    } else if is_typeof_call(&expected_value) {
        (expected_value, object_value.to_trimmed_text())
    } else {
        return Ok(None);
    };

    // Extract the argument to typeof()
    let call = unwrap_or_return_none!(typeof_call.as_r_call());
    let inner_args = call.arguments()?.items();
    let inner_arg = unwrap_or_return_none!(get_arg_by_name_then_position(&inner_args, "x", 1));
    let inner_value = unwrap_or_return_none!(inner_arg.value());
    let inner_text = inner_value.to_trimmed_text();

    // Preserve namespace prefix if present
    let function = ast.function()?;
    let namespace_prefix = get_function_namespace_prefix(function).unwrap_or_default();

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "expect_type".to_string(),
            format!("`{}(typeof(x), t)` can be hard to read.", function_name),
            Some("Use `expect_type(x, t)` instead.".to_string()),
        ),
        range,
        Fix {
            content: format!(
                "{}expect_type({}, {})",
                namespace_prefix, inner_text, type_text
            ),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}

fn check_expect_true_is_type(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let args = ast.arguments()?.items();

    // expect_type() doesn't have info= or label= arguments
    // If there are more than 1 argument, skip the lint
    if args.iter().count() > 1 {
        return Ok(None);
    }

    let object = unwrap_or_return_none!(get_arg_by_name_then_position(&args, "object", 1));
    let object_value = unwrap_or_return_none!(object.value());

    // Check if it's a call to an is.<type>() function
    let call = unwrap_or_return_none!(object_value.as_r_call());
    let function = call.function()?;
    let function_name = get_function_name(function);

    // Map is.<type> to type string, return None if not a valid type function
    let type_str = match function_name.as_str() {
        "is.logical" => "\"logical\"",
        "is.integer" => "\"integer\"",
        "is.double" => "\"double\"",
        "is.complex" => "\"complex\"",
        "is.character" => "\"character\"",
        "is.raw" => "\"raw\"",
        "is.list" => "\"list\"",
        "is.null" => "\"NULL\"",
        "is.symbol" => "\"symbol\"",
        "is.expression" => "\"expression\"",
        "is.language" => "\"language\"",
        "is.environment" => "\"environment\"",
        "is.pairlist" => "\"pairlist\"",
        _ => return Ok(None),
    };

    // Get the argument to is.<type>()
    let inner_args = call.arguments()?.items();
    let inner_arg = unwrap_or_return_none!(get_arg_by_name_then_position(&inner_args, "x", 1));
    let inner_value = unwrap_or_return_none!(inner_arg.value());
    let inner_text = inner_value.to_trimmed_text();

    // Preserve namespace prefix if present
    let outer_function = ast.function()?;
    let namespace_prefix = get_function_namespace_prefix(outer_function).unwrap_or_default();

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "expect_type".to_string(),
            "`expect_true(is.<t>(x))` can be hard to read.".to_string(),
            Some("Use `expect_type(x, t)` instead.".to_string()),
        ),
        range,
        Fix {
            content: format!(
                "{}expect_type({}, {})",
                namespace_prefix, inner_text, type_str
            ),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}

fn is_typeof_call(expr: &AnyRExpression) -> bool {
    if let Some(call) = expr.as_r_call() {
        if let Ok(function) = call.function() {
            return get_function_name(function) == "typeof";
        }
    }
    false
}
