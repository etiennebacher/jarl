use crate::diagnostic::*;
use crate::utils::get_arg_by_position;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct ClassEquals;

/// ## What it does
///
/// Checks for usage of `class(...) == "some_class"` and
/// `class(...) %in% "some_class"`.
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
/// class(x) == "lm"
/// ```
///
/// Use instead:
/// ```r
/// x <- lm(drat ~ mpg, mtcars)
/// class(x) <- c("my_class", class(x))
///
/// inherits(x, "lm")
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
        "Use `inherits(x, 'class')` instead of comparing `class(x)` with `==` or `%in%`."
            .to_string()
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
        },
    );
    Ok(Some(diagnostic))
}
