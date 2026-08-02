use crate::diagnostic::*;
use crate::utils::{
    get_arg_by_name_then_position, get_arg_by_position, get_function_name,
    get_function_namespace_prefix, node_contains_comments,
};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

/// Version added: 0.3.0
///
/// ## What it does
///
/// Checks for usage of `expect_equal(class(x), "y")`,
/// `expect_identical(class(x), "y")`, selected
/// `expect_true(is.<class>(x))`, and `expect_true(inherits(x, "y"))` calls.
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
/// This rule has a safe automatic fix for statically supported class names.
/// Dynamic class expressions are reported without an automatic fix because they
/// could contain classes that are not supported by `expect_s3_class()`.
///
/// This rule doesn't report cases where:
///
/// * the `is.*()` predicate does not test an S3 class. For example, `is.matrix(x)` does
///   not imply that `x` is an S3 object.
///
/// * `expect_s3_class()` would fail, such as:
///   ```r
///   testthat::expect_s3_class(list(1), "list")
///   testthat::expect_s3_class(1L, "integer")
///   ```
///   For those cases, it is recommended to use `expect_type()` instead.
///
/// Finally, the intent of the test cannot be inferred with the code only, so
/// the user will have to add `exact = TRUE` if necessary.
///
/// ## Example
///
/// ```r
/// expect_equal(class(x), "data.frame")
/// expect_identical(class(x), "Date")
/// expect_true(is.factor(x))
/// expect_true(inherits(x, "foo"))
/// ```
///
/// Use instead:
/// ```r
/// expect_s3_class(x, "data.frame")
/// expect_s3_class(x, "Date")
/// expect_s3_class(x, "factor")
/// expect_s3_class(x, "foo")
/// ```
pub fn expect_s3_class(ast: &RCall, fn_name: &str) -> anyhow::Result<Option<Diagnostic>> {
    match fn_name {
        "expect_equal" | "expect_identical" => check_expect_class_comparison(ast, fn_name),
        "expect_true" => check_expect_true_class(ast),
        _ => Ok(None),
    }
}

/// Checks comparisons such as `expect_equal(class(x), "data.frame")`.
fn check_expect_class_comparison(
    ast: &RCall,
    function_name: &str,
) -> anyhow::Result<Option<Diagnostic>> {
    let arguments = ast.arguments()?.items();

    // The replacement cannot preserve additional expectation arguments.
    if arguments.iter().count() != 2 {
        return Ok(None);
    }

    let object_argument =
        unwrap_or_return_none!(get_arg_by_name_then_position(&arguments, "object", 1));
    let expected_position = if object_argument.name_clause().is_some() {
        1
    } else {
        2
    };
    let expected_argument = unwrap_or_return_none!(get_arg_by_name_then_position(
        &arguments,
        "expected",
        expected_position,
    ));

    let object_value = unwrap_or_return_none!(object_argument.value());
    let expected_value = unwrap_or_return_none!(expected_argument.value());
    let linted_text = format!(
        "{function_name}({}, {})",
        object_value.to_trimmed_text(),
        expected_value.to_trimmed_text()
    );

    // Find patterns like `expect_equal(class(x), 'y')` and `expect_equal('y', class(x))`.
    let (class_call, class_expression) = if let Some(call) = as_class_call(&object_value)? {
        (call, expected_value)
    } else if let Some(call) = as_class_call(&expected_value)? {
        (call, object_value)
    } else {
        return Ok(None);
    };

    let can_fix = match classify_class_expression(&class_expression) {
        ClassExpressionKind::SupportedLiteral => true,
        ClassExpressionKind::UnsupportedLiteral => return Ok(None),
        ClassExpressionKind::Dynamic => false,
    };

    // Extract the argument of class()
    let class_arguments = class_call.arguments()?.items();
    let class_object_argument =
        unwrap_or_return_none!(get_arg_by_name_then_position(&class_arguments, "x", 1));
    let class_object = unwrap_or_return_none!(class_object_argument.value());

    let object_text = class_object.to_trimmed_text();
    let class_text = class_expression.to_trimmed_text();
    let replacement = format!("expect_s3_class({object_text}, {class_text})");

    // Preserve namespace prefix if present
    let function = ast.function()?;
    let namespace_prefix = get_function_namespace_prefix(function).unwrap_or_default();

    let range = ast.syntax().text_trimmed_range();

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            "expect_s3_class".to_string(),
            format!("`{linted_text}` may fail if `{object_text}` gets more classes in the future."),
            Some(format!("Use `{replacement}` instead.")),
        ),
        range,
        if can_fix {
            Fix {
                content: format!(
                    "{}expect_s3_class({}, {})",
                    namespace_prefix, object_text, class_text
                ),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            }
        } else {
            Fix::empty()
        },
    )))
}

/// Checks `expect_true()` calls containing `inherits()` or a known S3 predicate,
/// such as `expect_true(inherits(x, "foo"))` and `expect_true(is.factor(x))`.
fn check_expect_true_class(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let arguments = ast.arguments()?.items();

    // skip patterns like `expect_true(is.data.frame(x), info = "context")`
    if arguments.iter().count() != 1 {
        return Ok(None);
    }

    let object_argument =
        unwrap_or_return_none!(get_arg_by_name_then_position(&arguments, "object", 1));
    let object_value = unwrap_or_return_none!(object_argument.value());
    let predicate_call = unwrap_or_return_none!(object_value.as_r_call());
    let predicate_name = get_function_name(predicate_call.function()?);

    let predicate_arguments = predicate_call.arguments()?.items();
    let class_check = match predicate_name.as_str() {
        "inherits" => unwrap_or_return_none!(extract_inherits_check(&predicate_arguments)?),
        name if S3_CLASS_PREDICATES.contains(&name) => {
            unwrap_or_return_none!(extract_predicate_check(name, &predicate_arguments)?)
        }
        _ => return Ok(None),
    };

    let ClassCheck { object_text, class_text, can_fix } = class_check;
    let replacement = format!("expect_s3_class({object_text}, {class_text})");
    let linted_text = format!("expect_true({})", predicate_call.to_trimmed_text());

    let function = ast.function()?;
    let namespace_prefix = get_function_namespace_prefix(function).unwrap_or_default();
    let range = ast.syntax().text_trimmed_range();

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            "expect_s3_class".to_string(),
            format!("`{replacement}` is better than `{linted_text}`."),
            Some(format!("Use `{replacement}` instead.")),
        ),
        range,
        if can_fix {
            Fix {
                content: format!("{namespace_prefix}{replacement}"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            }
        } else {
            Fix::empty()
        },
    )))
}

/// The object, expected class, and fix safety extracted from a class check.
///
/// For example, `inherits(x, "foo")` produces `x`, `"foo"`, and a safe fix.
struct ClassCheck {
    object_text: String,
    class_text: String,
    can_fix: bool,
}

/// Extracts the object and class from a call such as `inherits(x, "foo")`.
fn extract_inherits_check(arguments: &RArgumentList) -> anyhow::Result<Option<ClassCheck>> {
    if arguments.iter().count() != 2 {
        return Ok(None);
    }

    let object_argument = unwrap_or_return_none!(get_arg_by_name_then_position(arguments, "x", 1));
    let class_position = if object_argument.name_clause().is_some() {
        1
    } else {
        2
    };
    let class_argument = unwrap_or_return_none!(get_arg_by_name_then_position(
        arguments,
        "what",
        class_position,
    ));
    let object = unwrap_or_return_none!(object_argument.value());
    let class = unwrap_or_return_none!(class_argument.value());
    let can_fix = match classify_class_expression(&class) {
        ClassExpressionKind::SupportedLiteral => true,
        ClassExpressionKind::UnsupportedLiteral => return Ok(None),
        ClassExpressionKind::Dynamic => false,
    };

    Ok(Some(ClassCheck {
        object_text: object.to_trimmed_text().to_string(),
        class_text: class.to_trimmed_text().to_string(),
        can_fix,
    }))
}

/// Converts a known predicate such as `is.factor(x)` into object `x` and class
/// `"factor"`.
fn extract_predicate_check(
    predicate_name: &str,
    arguments: &RArgumentList,
) -> anyhow::Result<Option<ClassCheck>> {
    if arguments.iter().count() != 1 {
        return Ok(None);
    }

    let argument = unwrap_or_return_none!(get_arg_by_position(arguments, 1));
    let object = unwrap_or_return_none!(argument.value());
    let class_name = unwrap_or_return_none!(predicate_name.strip_prefix("is."));

    Ok(Some(ClassCheck {
        object_text: object.to_trimmed_text().to_string(),
        class_text: format!("\"{class_name}\""),
        can_fix: true,
    }))
}

/// Returns the call when an expression has the form `class(x)`.
fn as_class_call(expression: &AnyRExpression) -> anyhow::Result<Option<&RCall>> {
    if let Some(call) = expression.as_r_call()
        && get_function_name(call.function()?) == "class"
    {
        Ok(Some(call))
    } else {
        Ok(None)
    }
}

// This list follows lintr's manually curated set of predicates that test S3 classes.
// See https://github.com/r-lib/lintr/blob/main/R/expect_s3_class_linter.R
const S3_CLASS_PREDICATES: &[&str] = &[
    "is.data.frame",
    "is.factor",
    "is.numeric_version",
    "is.ordered",
    "is.package_version",
    "is.qr",
    "is.table",
    "is.relistable",
    "is.raster",
    "is.tclObj",
    "is.tkwin",
    "is.grob",
    "is.unit",
    "is.mts",
    "is.stepfun",
    "is.ts",
    "is.tskernel",
];

// https://github.com/wch/r-source/blob/e945946d165f3d9d2afa2e214a39aa4af61be45c/src/main/util.c#L209-L240
// Link provided in https://github.com/etiennebacher/jarl/issues/232#issuecomment-3632266565
const NON_S3_CLASSES: &[&str] = &[
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
    "dgCMatrix",
    "dgRMatrix",
    "dgeMatrix",
    "dgTMatrix",
    "array",
    "function",
];

/// Classifies a class expression according to whether the rule can safely fix it.
///
/// For example, `"Date"` is supported, `"integer"` is unsupported, and a
/// variable such as `expected_class` is dynamic.
enum ClassExpressionKind {
    /// A string literal naming a class supported by `expect_s3_class()`.
    SupportedLiteral,
    /// A string literal naming a class that is not an S3 class.
    UnsupportedLiteral,
    /// An expression whose class value cannot be determined statically.
    Dynamic,
}

/// Classifies the class argument used by `expect_s3_class()`.
fn classify_class_expression(expression: &AnyRExpression) -> ClassExpressionKind {
    let Some(string) = expression
        .as_any_r_value()
        .and_then(|value| value.as_r_string_value())
    else {
        return ClassExpressionKind::Dynamic;
    };

    let content = string.content_token();
    let class_name = content.as_ref().map_or("", |token| token.text_trimmed());

    if NON_S3_CLASSES.contains(&class_name) {
        ClassExpressionKind::UnsupportedLiteral
    } else {
        ClassExpressionKind::SupportedLiteral
    }
}
