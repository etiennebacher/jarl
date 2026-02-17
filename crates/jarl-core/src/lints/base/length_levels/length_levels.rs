use crate::diagnostic::*;
use crate::utils::{get_nested_functions_content, node_contains_comments};
use air_r_syntax::*;
pub struct LengthLevels;

/// ## What it does
///
/// Check for `length(levels(...))` and replace it with `nlevels(...)`.
///
/// ## Why is this bad?
///
/// `length(levels(...))` is harder to read `nlevels(...)`.
///
/// Internally, `nlevels()` calls `length(levels(...))` so there are no
/// performance gains.
///
/// ## Example
///
/// ```r
/// x <- factor(1:3)
/// length(levels(x))
/// ```
///
/// Use instead:
/// ```r
/// x <- factor(1:3)
/// nlevels(x)
/// ```
impl Violation for LengthLevels {
    fn name(&self) -> String {
        "length_levels".to_string()
    }
    fn body(&self) -> String {
        "`length(levels(...))` is less readable than `nlevels(...)`.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `nlevels(...)` instead.".to_string())
    }
}

pub fn length_levels(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let (inner_content, outer_syntax) =
        unwrap_or_return_none!(get_nested_functions_content(ast, "length", "levels")?);

    let range = outer_syntax.text_trimmed_range();
    Ok(Some(Diagnostic::new(
        LengthLevels,
        range,
        Fix {
            content: format!("nlevels({inner_content})"),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(&outer_syntax),
        },
    )))
}
