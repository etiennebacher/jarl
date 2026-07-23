use crate::diagnostic::*;
use crate::utils::{
    get_arg_by_name_then_position, get_function_name, get_function_namespace_prefix,
    node_contains_comments,
};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for usage of `expect_true(is(x, "y"))`.
///
/// ## Why is this bad?
///
/// `expect_s4_class()` is designed specifically for testing the class of S4
/// objects. It makes the intent clearer and provides better error messages when
/// the test fails.
///
/// This rule is **disabled by default**. Select it either with the rule name
/// `"expect_s4_class"` or with the rule group `"TESTTHAT"`.
///
/// This rule has a safe automatic fix but doesn't report calls that pass
/// `info` or `label` to `expect_true()`.
///
/// ## Example
///
/// ```r
/// expect_true(is(x, "Matrix"))
/// ```
///
/// Use instead:
/// ```r
/// expect_s4_class(x, "Matrix")
/// ```
pub fn expect_s4_class(ast: &RCall, fn_name: &str) -> anyhow::Result<Option<Diagnostic>> {
    // Only check expect_true
    if fn_name != "expect_true" {
        return Ok(None);
    }

    let arguments = ast.arguments()?.items();
    if arguments.iter().count() != 1 {
        return Ok(None);
    }

    let object = unwrap_or_return_none!(get_arg_by_name_then_position(&arguments, "object", 1));
    let object_value = unwrap_or_return_none!(object.value());
    let is_call = unwrap_or_return_none!(object_value.as_r_call());

    // Only check is()
    if get_function_name(is_call.function()?) != "is" {
        return Ok(None);
    }

    // Only check is() with two arguments
    let is_arguments = is_call.arguments()?.items();
    if is_arguments.iter().count() != 2 {
        return Ok(None);
    }

    let object = unwrap_or_return_none!(get_arg_by_name_then_position(&is_arguments, "object", 1));
    let class = unwrap_or_return_none!(get_arg_by_name_then_position(&is_arguments, "class2", 2));
    let object_value = unwrap_or_return_none!(object.value());
    let class_value = unwrap_or_return_none!(class.value());

    let object_text = object_value.to_trimmed_text();
    let class_text = class_value.to_trimmed_text();
    let replacement = format!("expect_s4_class({object_text}, {class_text})");
    let linted_text = format!("expect_true({})", is_call.to_trimmed_text());

    let namespace_prefix = get_function_namespace_prefix(ast.function()?).unwrap_or_default();
    let range = ast.syntax().text_trimmed_range();

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            "expect_s4_class".to_string(),
            format!("`{replacement}` is better than `{linted_text}`."),
            Some(format!("Use `{replacement}` instead.")),
        ),
        range,
        Fix {
            content: format!(
                "{}expect_s4_class({}, {})",
                namespace_prefix, object_text, class_text
            ),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    )))
}
