use crate::diagnostic::*;
use crate::utils::{get_arg_by_position, get_function_name, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for usage of `class(...) == "some_class"`,
/// `class(...) %in% "some_class"`, and `identical(class(...), "some_class")`.
///
/// For `==` and `%in%` operators, the only cases that are flagged (and potentially
/// fixed) are cases that:
///
/// - happen in the condition part of an `if ()` statement or of a `while ()`
///   statement,
/// - and are not nested in other calls.
///
/// For example, `if (class(x) == "foo")` would be reported, but not
/// `if (my_function(class(x) == "foo"))`.
///
/// For `identical()` calls, all cases are flagged regardless of context.
///
/// ## Why is this bad?
///
/// An R object can have several classes. Therefore,
/// `class(...) == "some_class"` would return a logical vector with as many
/// values as the object has classes, which is rarely desirable.
///
/// It is better to use `inherits(..., "some_class")` instead. `inherits()`
/// checks whether any of the object's classes match the desired class.
///
/// The same rationale applies to `class(...) %in% "some_class"`. Similarly,
/// `identical(class(...), "some_class")` would break if a class is added or
/// removed to the object being tested.
///
/// ## Example
///
/// ```r
/// x <- lm(drat ~ mpg, mtcars)
/// class(x) <- c("my_class", class(x))
///
/// if (class(x) == "lm") {
///   # <do something>
/// }
///
/// identical(class(x), "foo")
/// ```
///
/// Use instead:
/// ```r
/// x <- lm(drat ~ mpg, mtcars)
/// class(x) <- c("my_class", class(x))
///
/// if (inherits(x, "lm")) {
///   # <do something>
/// }
///
/// inherits(x, "foo")
/// ```
///
/// ## References
///
/// See `?inherits`
pub fn class_equals(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let operator = operator?;
    let left = left?;
    let right = right?;

    if operator.kind() != RSyntaxKind::EQUAL2
        && operator.kind() != RSyntaxKind::NOT_EQUAL
        && operator.text_trimmed() != "%in%"
    {
        return Ok(None);
    };

    // We want to skip cases like the following where we don't know exactly
    // how the `class(x) == "foo"` is used for.
    // ```r
    // x <- 1
    // class(x) <- c("foo", "bar")
    //
    // which_to_subset <- class(x) == "foo"
    // which_to_subset_2 <- inherits(x, "foo")
    //
    // class(x)[which_to_subset]
    // #> [1] "foo"
    // class(x)[which_to_subset_2]
    // #> [1] "foo" "bar"
    // ```
    //
    // We report only cases where we know this is incorrect:
    // - in the condition of an RIfStatement;
    // - in the condition of an RWhileStatement.
    if !is_in_condition_context(ast.syntax()) {
        return Ok(None);
    }

    let (fun_content, class_name) = match extract_class_and_string(left, right) {
        Some(result) => result,
        None => return Ok(None),
    };

    let fun_name = if operator.kind() == RSyntaxKind::EQUAL2 || operator.text_trimmed() == "%in%" {
        "inherits"
    } else {
        "!inherits"
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "class_equals".to_string(),
            "Comparing `class(x)` with `==` or `%in%` can be problematic.".to_string(),
            Some("Use `inherits(x, 'a')` instead.".to_string()),
        ),
        range,
        Fix {
            content: format!("{}({}, {})", fun_name, fun_content, class_name),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );
    Ok(Some(diagnostic))
}

pub fn class_identical(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let function_name = get_function_name(function);
    if function_name != "identical" {
        return Ok(None);
    }

    // Note: Unlike class_equals with == and %in%, we report identical() in all contexts,
    // not just in if/while conditions, because identical() is almost never the right choice
    // when comparing class() output with a string.

    let args = ast.arguments()?.items();
    let first_arg = unwrap_or_return_none!(get_arg_by_position(&args, 1));
    let second_arg = unwrap_or_return_none!(get_arg_by_position(&args, 2));

    let first_value = unwrap_or_return_none!(first_arg.value());
    let second_value = unwrap_or_return_none!(second_arg.value());

    // Extract class() and string
    let (fun_content, class_name) = match extract_class_and_string(first_value, second_value) {
        Some(result) => result,
        None => return Ok(None),
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "class_equals".to_string(),
            "Using `identical(class(x), 'a')` can be problematic.".to_string(),
            Some("Use `inherits(x, 'a')` instead.".to_string()),
        ),
        range,
        Fix {
            content: format!("inherits({}, {})", fun_content, class_name),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );
    Ok(Some(diagnostic))
}

fn is_in_condition_context(node: &RSyntaxNode) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };

    // The `condition` part of an `RIfStatement` is always the 3rd node (index 2):
    // IF_KW - L_PAREN - [condition] - R_PAREN - [consequence]
    let parent_is_if = parent.kind() == RSyntaxKind::R_IF_STATEMENT && node.index() == 2;

    // The `condition` part of an `RWhileStatement` is always the 3rd node (index 2):
    // WHILE_KW - L_PAREN - [condition] - R_PAREN - [consequence]
    let parent_is_while = parent.kind() == RSyntaxKind::R_WHILE_STATEMENT && node.index() == 2;

    parent_is_if || parent_is_while
}

/// Extract class() call and string literal from two expressions
/// Returns (class_call_content, class_name) where:
/// - class_call_content is the first argument to class()
/// - class_name is the string literal
fn extract_class_and_string(
    left: AnyRExpression,
    right: AnyRExpression,
) -> Option<(String, String)> {
    let mut left_is_class = false;
    let mut right_is_class = false;

    // Check if left is a class() call or a string
    if let Some(left_call) = left.as_r_call() {
        let left_fn = left_call.function().ok()?;
        let left_fn_name = get_function_name(left_fn);
        if left_fn_name != "class" {
            return None;
        }
        left_is_class = true;
    } else if let Some(left_val) = left.as_any_r_value() {
        left_val.as_r_string_value()?;
    } else {
        return None;
    }

    // Check if right is a class() call or a string
    if let Some(right_call) = right.as_r_call() {
        let right_fn = right_call.function().ok()?;
        let right_fn_name = get_function_name(right_fn);
        if right_fn_name != "class" {
            return None;
        }
        right_is_class = true;
    } else if let Some(right_val) = right.as_any_r_value() {
        right_val.as_r_string_value()?;
    } else {
        return None;
    }

    // We need exactly one class() and one string
    let left_is_string = !left_is_class;
    let right_is_string = !right_is_class;

    if !(left_is_class && right_is_string || left_is_string && right_is_class) {
        return None;
    }

    // Extract the content
    let (fun_content, class_name) = if left_is_class {
        let args = left.as_r_call()?.arguments().ok()?.items();
        let content = get_arg_by_position(&args, 1)?.to_trimmed_text();
        let name = right.to_trimmed_text();
        (content, name)
    } else {
        let args = right.as_r_call()?.arguments().ok()?.items();
        let content = get_arg_by_position(&args, 1)?.to_trimmed_text();
        let name = left.to_trimmed_text();
        (content, name)
    };

    Some((fun_content.to_string(), class_name.to_string()))
}
