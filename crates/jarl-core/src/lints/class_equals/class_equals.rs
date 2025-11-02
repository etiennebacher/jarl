use crate::diagnostic::*;
use crate::utils::{get_arg_by_position, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct ClassEquals;

/// ## What it does
///
/// Checks for usage of `class(...) == "some_class"` and
/// `class(...) %in% "some_class"`. The only cases that are flagged (and
/// potentially fixed) are cases that:
///
/// - happen in the condition part of an `if ()` statement or of a `while ()`
///   statement,
/// - and are not nested in other calls.
///
/// For example, `if (class(x) == "foo")` would be reported, but not
/// `if (my_function(class(x) == "foo"))`.
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
/// The same rationale applies to `class(...) %in% "some_class"`.
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
/// ```
///
/// ## References
///
/// See `?inherits`
impl Violation for ClassEquals {
    fn name(&self) -> String {
        "class_equals".to_string()
    }
    fn body(&self) -> String {
        "Comparing `class(x)` with `==` or `%in%` can be problematic.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `inherits(x, 'class')` instead.".to_string())
    }
}

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
    //
    //
    // The `condition` part of an `RIfStatement` is always the 3th node
    // (index 2):
    // IF_KW - L_PAREN - [condition] - R_PAREN - [consequence]
    let parent_is_if = ast.syntax().parent().unwrap().kind() == RSyntaxKind::R_IF_STATEMENT
        && ast.syntax().index() == 2;
    // The `condition` part of an `RWhileStatement` is always the 3th node
    // (index 2):
    // WHILE_KW - L_PAREN - [condition] - R_PAREN - [consequence]
    let parent_is_while = ast.syntax().parent().unwrap().kind() == RSyntaxKind::R_WHILE_STATEMENT
        && ast.syntax().index() == 2;

    if !parent_is_if && !parent_is_while {
        return Ok(None);
    }

    let mut left_is_class = false;
    let mut right_is_class = false;

    // Return early if left is neither a function call nor a string.
    if let Some(left) = left.as_r_call() {
        if left.function()?.to_trimmed_text() != "class" {
            return Ok(None);
        }
        left_is_class = true;
    } else if let Some(left) = left.as_any_r_value() {
        if let Some(_left) = left.as_r_string_value() {
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    }

    // Return early if right is neither a function call nor a string.
    if let Some(right) = right.as_r_call() {
        if right.function()?.to_trimmed_text() != "class" {
            return Ok(None);
        }
        right_is_class = true;
    } else if let Some(right) = right.as_any_r_value() {
        if let Some(_right) = right.as_r_string_value() {
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    }

    // At this point, we know they're either string or class().
    let left_is_string = !left_is_class;
    let right_is_string = !right_is_class;

    if !(left_is_class && right_is_string) & !(left_is_string && right_is_class) {
        return Ok(None);
    }

    let fun_name = if operator.kind() == RSyntaxKind::EQUAL2 || operator.text_trimmed() == "%in%" {
        "inherits"
    } else {
        "!inherits"
    };

    let fun_content;
    let class_name;

    if left_is_class {
        let args = left.as_r_call().unwrap().arguments()?.items();
        fun_content = get_arg_by_position(&args, 1).map(|x| x.to_trimmed_text());
        class_name = right.to_trimmed_text();
    } else {
        let args = right.as_r_call().unwrap().arguments()?.items();
        fun_content = get_arg_by_position(&args, 1).map(|x| x.to_trimmed_text());
        class_name = left.to_trimmed_text();
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ClassEquals,
        range,
        Fix {
            content: format!("{}({}, {})", fun_name, fun_content.unwrap(), class_name),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );
    Ok(Some(diagnostic))
}
