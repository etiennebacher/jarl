use crate::diagnostic::*;
use crate::utils::get_nested_functions_content;
use air_r_syntax::*;
use biome_rowan::AstNode;
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
        "Use `nlevels(...)` instead of `length(levels(...))`.".to_string()
    }
}

pub fn length_levels(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let inner_content = get_nested_functions_content(ast, "length", "levels")?;

    if let Some(inner_content) = inner_content {
        let range = ast.syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(
            LengthLevels,
            range,
            Fix {
                content: format!("nlevels({inner_content})"),
                start: range.start().into(),
                end: range.end().into(),
            },
        );
        return Ok(Some(diagnostic));
    }
    Ok(None)
}
