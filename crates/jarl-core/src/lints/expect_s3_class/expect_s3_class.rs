use crate::diagnostic::*;
use crate::utils::{
    get_arg_by_name_then_position, get_function_name, get_function_namespace_prefix,
    node_contains_comments,
};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

/// ## What it does
///
/// Checks for usage of `expect_equal(class(x), "y")` and
/// `expect_identical(class(x), "y")`.
///
/// ## Why is this bad?
///
/// `expect_equal(class(x), "y")` will fail if `x` gets more classes in the future,
/// even if `"y"` is still one of those classes. `expect_s3_class(x, "y")`
/// is more robust because the test success doesn't depend on the number or
/// on the order of classes of `x`. This function also gives clearer error
/// messages in case of failure.
///
/// To test that `x` only has the class `"y"`, then one can use
/// `expect_s3_class(x, "y", exact = TRUE)`.
///
/// This rule is **disabled by default**. Select it either with the rule name
/// `"expect_s3_class"` or with the rule group `"TESTTHAT"`.
///
/// This rule has a safe automatic fix but doesn't report cases where:
///
/// * `expect_s3_class()` would fail, such as:
///   ```r
///   testthat::expect_s3_class(list(1), "list")
///   testthat::expect_s3_class(1L, "integer")
///   ```
///   For those cases, it is recommended to use `expect_type()` instead.
///
/// * the `expected` object could have multiple values, such as:
///   ```r
///   testthat::expect_equal(class(x), c("foo", "bar"))
///   testthat::expect_equal(class(x), vec_of_classes)
///   ```
///
/// Finally, the intent of the test cannot be inferred with the code only, so
/// the user will have to add `exact = TRUE` if necessary.
///
/// ## Example
///
/// ```r
/// expect_equal(class(x), "data.frame")
/// expect_identical(class(x), "Date")
/// ```
///
/// Use instead:
/// ```r
/// expect_s3_class(x, "data.frame")
/// expect_s3_class(x, "Date")
/// ```
pub fn expect_s3_class(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let function_name = get_function_name(function.clone());

    // Only check expect_equal and expect_identical
    if function_name != "expect_equal" && function_name != "expect_identical" {
        return Ok(None);
    }

    let args = ast.arguments()?.items();

    let object = unwrap_or_return_none!(get_arg_by_name_then_position(&args, "object", 1));
    let expected = unwrap_or_return_none!(get_arg_by_name_then_position(&args, "expected", 2));

    // Don't know how to handle argument `label` for instance.
    if args.iter().count() > 2 {
        return Ok(None);
    }

    let object_value = unwrap_or_return_none!(object.value());
    let expected_value = unwrap_or_return_none!(expected.value());

    // Find patterns like `expect_equal(class(x), 'y')` and `expect_equal('y', class(x))`.
    let (class_arg, other_arg) = if let Some(object_call) = object_value.as_r_call() {
        let obj_fn = object_call.function()?;
        let obj_fn_name = get_function_name(obj_fn);

        if obj_fn_name == "class" {
            (object_call, expected_value)
        } else {
            return Ok(None);
        }
    } else if let Some(expected_call) = expected_value.as_r_call() {
        let exp_fn = expected_call.function()?;
        let exp_fn_name = get_function_name(exp_fn);

        if exp_fn_name == "class" {
            (expected_call, object_value)
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    };

    if !check_class_is_s3(&other_arg) {
        return Ok(None);
    }

    // Extract the argument of class()
    let class_args = class_arg.arguments()?.items();
    let class_x_arg = unwrap_or_return_none!(get_arg_by_name_then_position(&class_args, "x", 1));
    let class_x_value = unwrap_or_return_none!(class_x_arg.value());

    let x_text = class_x_value.to_trimmed_text();
    let n_text = other_arg.to_trimmed_text();

    // Preserve namespace prefix if present
    let namespace_prefix = get_function_namespace_prefix(function).unwrap_or_default();

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "expect_s3_class".to_string(),
            format!(
                "`{}(class(x), 'y')` may fail if `x` gets more classes in the future.",
                function_name
            ),
            Some("Use `expect_s3_class(x, 'y')` instead.".to_string()),
        ),
        range,
        Fix {
            content: format!(
                "{}expect_s3_class({}, {})",
                namespace_prefix, x_text, n_text
            ),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}

// https://github.com/wch/r-source/blob/e945946d165f3d9d2afa2e214a39aa4af61be45c/src/main/util.c#L209-L240
// Link provided in https://github.com/etiennebacher/jarl/issues/232#issuecomment-3632266565
pub static IGNORED_CLASSES: &[&str] = &[
    "NULL",
    "symbol",
    "pairlist",
    "closure",
    "environment",
    "promise",
    "language",
    "special",
    "builtin",
    "char",
    "logical",
    "integer",
    "double",
    "complex",
    "character",
    "...",
    "any",
    "expression",
    "list",
    "externalptr",
    "bytecode",
    "weakref",
    "raw",
    "S4",
    "object",
    "numeric",
    "name",
    // Not in the linked file, but `expect_s3_class(list(1), "list")` fails.
    "list",
    // See `?class`
    "matrix",
    "array",
    "function",
];

fn check_class_is_s3(x: &AnyRExpression) -> bool {
    if let Some(x) = x.as_any_r_value()
        && let Some(x) = x.as_r_string_value()
    {
        !IGNORED_CLASSES.contains(
            &x.to_trimmed_text()
                .to_string()
                .replace("\"", "")
                .replace("'", "")
                .as_str(),
        )
    } else {
        false
    }
}
