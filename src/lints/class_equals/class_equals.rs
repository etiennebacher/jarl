use crate::message::*;
use crate::utils::get_first_arg;
use air_r_syntax::*;
use anyhow::Result;
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

pub fn class_equals(ast: &RBinaryExpression) -> Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let operator = operator?;

    if operator.kind() != RSyntaxKind::EQUAL2
        && operator.kind() != RSyntaxKind::NOT_EQUAL
        && operator.text_trimmed() != "%in%"
    {
        return Ok(None);
    };

    let lhs = left?.into_syntax();
    let rhs = right?.into_syntax();

    let left_is_class = lhs
        .first_child()
        .map(|x| x.text_trimmed() == "class")
        .unwrap_or(false);
    let right_is_class = rhs
        .first_child()
        .map(|x| x.text_trimmed() == "class")
        .unwrap_or(false);
    let left_is_string = lhs.kind() == RSyntaxKind::R_STRING_VALUE;
    let right_is_string = rhs.kind() == RSyntaxKind::R_STRING_VALUE;

    if (!left_is_class && !right_is_class) || (!left_is_string && !right_is_string) {
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
        fun_content = get_first_arg(&lhs).map(|x| x.text_trimmed());
        class_name = rhs.text_trimmed();
    } else {
        fun_content = get_first_arg(&rhs).map(|x| x.text_trimmed());
        class_name = lhs.text_trimmed();
    };

    let range = ast.clone().into_syntax().text_trimmed_range();
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
