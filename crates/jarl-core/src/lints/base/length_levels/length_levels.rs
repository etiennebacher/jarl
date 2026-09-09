use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{get_nested_functions_content, node_contains_comments};
use air_r_syntax::*;
pub struct LengthLevels;

/// Version added: 0.0.8
///
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
    fn rule(&self) -> Rule {
        Rule::LengthLevels
    }
    fn body(&self) -> String {
        "`length(levels(...))` is less readable than `nlevels(...)`.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `nlevels(...)` instead.".to_string())
    }
}

pub fn length_levels(ast: &RCall, fn_name: &str) -> anyhow::Result<Option<Diagnostic>> {
    let (inner_content, outer_syntax) = unwrap_or_return_none!(get_nested_functions_content(
        ast, fn_name, "length", "levels"
    )?);

    let range = outer_syntax.text_trimmed_range();
    Ok(Some(Diagnostic::new(
        LengthLevels,
        range,
        Fix::new(
            range,
            format!("nlevels({inner_content})"),
            node_contains_comments(&outer_syntax),
        ),
    )))
}
