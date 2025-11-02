use crate::diagnostic::*;
use crate::utils::{get_nested_functions_content, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct AnyIsNa;

/// ## What it does
///
/// Checks for usage of `any(is.na(...))`.
///
/// ## Why is this bad?
///
/// `any(is.na(...))` is valid code but requires the evaluation of `is.na()` on
/// the entire input first.
///
/// There is a more efficient function in base R called `anyNA()` that is more
/// efficient, both in speed and memory used.
///
/// ## Example
///
/// ```r
/// x <- c(1:10000, NA)
/// any(is.na(x))
/// ```
///
/// Use instead:
/// ```r
/// x <- c(1:10000, NA)
/// anyNA(x)
/// ```
///
/// ## References
///
/// See `?anyNA`
impl Violation for AnyIsNa {
    fn name(&self) -> String {
        "any_is_na".to_string()
    }
    fn body(&self) -> String {
        "`any(is.na(...))` is inefficient.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `anyNA(...)` instead.".to_string())
    }
}

pub fn any_is_na(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let inner_content = get_nested_functions_content(ast, "any", "is.na")?;

    if let Some(inner_content) = inner_content {
        let range = ast.syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(
            AnyIsNa,
            range,
            Fix {
                content: format!("anyNA({inner_content})"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        );
        return Ok(Some(diagnostic));
    }

    Ok(None)
}
