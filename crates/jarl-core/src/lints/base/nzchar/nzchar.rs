use crate::diagnostic::*;
use crate::utils::node_contains_comments;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct NzChar;

/// ## What it does
///
/// Checks for usage of `x != ""` or `x == ""`
///  instead of `nzchar(x)` or `!nzchar(x)`.
///
/// ## Why is this bad?
/// `x == ""` is less efficient than `!nzchar(x)`
/// when x is a large vector of long strings. 
///
/// One crucial difference is in the default handling of `NA_character_`,
/// i.e., missing strings. `nzchar(NA_character_)` is TRUE,
/// while `NA_character_ == ""` is NA.
/// Therefore, for strict compatibility, use `nzchar(x, keepNA = TRUE)`.
/// If the input is known to be complete (no missing entries),
/// this argument can be dropped for conciseness.
///
/// This rule comes with a unsafe fix.
///
/// ## Example
///
/// ```r
/// x <- sample(c("abcdefghijklmn", "", "opqrstuvwyz"), 1e7, TRUE)
/// x[x == ""]
/// ```
///
/// Use instead:
/// ```r
/// x <- sample(c("abcdefghijklmn", "", "opqrstuvwyz"), 1e7, TRUE)
/// x[!nzchar(x)]
/// ```
///
/// ## References
///
/// See `?nzchar`
impl Violation for NzChar {
    fn name(&self) -> String {
        "nzchar".to_string()
    }
    fn body(&self) -> String {
        "`x == \"\"` is inefficient.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `!nzchar(x)` instead.".to_string())
    }
}

pub fn nzchar(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let left = left?;
    let operator = operator?;
    let right = right?;

    if operator.kind() != RSyntaxKind::EQUAL2 && operator.kind() != RSyntaxKind::NOT_EQUAL {
        return Ok(None);
    };

    let left_is_empty_string = left
        .to_trimmed_string()
        .trim_matches('"')
        .trim_matches('\'')
        .is_empty();
    let right_is_empty_string = right
        .to_trimmed_string()
        .trim_matches('"')
        .trim_matches('\'')
        .is_empty();

    if (left_is_empty_string && right_is_empty_string)
        || (!left_is_empty_string && !right_is_empty_string)
    {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();

    let replacement = if left_is_empty_string {
        right.to_trimmed_string()
    } else {
        left.to_trimmed_string()
    };

    let diagnostic = match operator.kind() {
        RSyntaxKind::EQUAL2 => Diagnostic::new(
            NzChar,
            range,
            Fix {
                content: format!("!nzchar({replacement})"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        ),
        RSyntaxKind::NOT_EQUAL => Diagnostic::new(
            NzChar,
            range,
            Fix {
                content: format!("nzchar({replacement})"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        ),
        _ => unreachable!("This case is an early return"),
    };

    Ok(Some(diagnostic))
}
