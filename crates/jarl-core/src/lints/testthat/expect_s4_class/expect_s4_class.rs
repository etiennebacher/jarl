use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{
    Formals, get_arg, get_function_name, get_function_namespace_prefix, node_contains_comments,
};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

const FORMALS_EXPECT_TRUE: Formals = &["object", "info", "label"];
const FORMALS_IS: Formals = &["object", "class2"];

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

    let object = unwrap_or_return_none!(get_arg(ast, FORMALS_EXPECT_TRUE, "object"));
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

    let object = unwrap_or_return_none!(get_arg(is_call, FORMALS_IS, "object"));
    let class = unwrap_or_return_none!(get_arg(is_call, FORMALS_IS, "class2"));
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
            Rule::TestthatExpectS4Class,
            format!("`{replacement}` is better than `{linted_text}`."),
            Some(format!("Use `{replacement}` instead.")),
        ),
        range,
        Fix::new(
            range,
            format!(
                "{}expect_s4_class({}, {})",
                namespace_prefix, object_text, class_text
            ),
            node_contains_comments(ast.syntax()),
        ),
    )))
}
