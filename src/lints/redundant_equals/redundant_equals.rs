use crate::diagnostic::*;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct RedundantEquals;

/// ## What it does
///
/// Checks for usage of `==` and `!=` where one of the sides of the operation
/// is `TRUE` or `FALSE`.
///
/// ## Why is this bad?
///
/// Testing `x == TRUE` is redundant if `x` is a logical vector. Wherever this
/// is used to improve readability, the solution should instead be to improve
/// the naming of the object to better indicate that its contents are logical.
/// This can be done using prefixes (is, has, can, etc.). For example,
/// `is_child`, `has_parent_supervision`, `can_watch_horror_movie` clarify
/// their logical nature, while `child`, `parent_supervision`,
/// `watch_horror_movie` don't.
///
/// ## Example
///
/// ```r
/// x <- c(TRUE, FALSE)
/// if (any(x == TRUE)) {
///   print("hi")
/// }
/// ```
///
/// Use instead:
/// ```r
/// x <- c(TRUE, FALSE)
/// if (any(x)) {
///   print("hi")
/// }
/// ```
impl Violation for RedundantEquals {
    fn name(&self) -> String {
        "redundant_equals".to_string()
    }
    fn body(&self) -> String {
        "Using == on a logical vector is redundant.".to_string()
    }
}

pub fn redundant_equals(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let operator = operator?;
    let left = left?;
    let right = right?;

    let left_is_true = &left.as_r_true_expression().is_some();
    let left_is_false = &left.as_r_false_expression().is_some();
    let right_is_true = &right.as_r_true_expression().is_some();
    let right_is_false = &right.as_r_false_expression().is_some();

    let diagnostic = match operator.kind() {
        RSyntaxKind::EQUAL2 => {
            let fix = if *left_is_true {
                right.into_syntax().text_trimmed().to_string()
            } else if *right_is_true {
                left.into_syntax().text_trimmed().to_string()
            } else if *left_is_false {
                format!("!{}", right.into_syntax().text_trimmed())
            } else if *right_is_false {
                format!("!{}", left.into_syntax().text_trimmed())
            } else {
                return Ok(None);
            };

            let range = ast.syntax().text_trimmed_range();
            Diagnostic::new(
                RedundantEquals,
                range,
                Fix {
                    content: fix,
                    start: range.start().into(),
                    end: range.end().into(),
                },
            )
        }
        RSyntaxKind::NOT_EQUAL => {
            let fix = if *left_is_true {
                format!("!{}", right.into_syntax().text_trimmed())
            } else if *right_is_true {
                format!("!{}", left.into_syntax().text_trimmed())
            } else if *left_is_false {
                right.into_syntax().text_trimmed().to_string()
            } else if *right_is_false {
                left.into_syntax().text_trimmed().to_string()
            } else {
                return Ok(None);
            };
            let range = ast.syntax().text_trimmed_range();
            Diagnostic::new(
                RedundantEquals,
                range,
                Fix {
                    content: fix,
                    start: range.start().into(),
                    end: range.end().into(),
                },
            )
        }
        _ => return Ok(None),
    };
    Ok(Some(diagnostic))
}
