use crate::message::*;
use crate::utils::get_function_name;
use air_r_syntax::*;
use anyhow::Result;
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

pub fn length_levels(ast: &RCall) -> Result<Option<Diagnostic>> {
    let RCallFields { function, arguments } = ast.as_fields();

    let function = function?;
    let outer_fn_name = get_function_name(function);

    if outer_fn_name != "length" {
        return Ok(None);
    }

    let items = arguments?.items();

    let unnamed_arg = items
        .into_iter()
        .find(|x| x.clone().unwrap().name_clause().is_none());

    let value = unnamed_arg.unwrap()?.value();

    if let Some(inner) = value
        && let Some(inner2) = inner.as_r_call()
    {
        let RCallFields { function, arguments } = inner2.as_fields();

        let function = function?;
        let inner_fn_name = get_function_name(function);

        if inner_fn_name != "levels" {
            return Ok(None);
        }

        let inner_content = arguments?.items().into_syntax().text();
        let range = ast.clone().into_syntax().text_trimmed_range();
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
