use crate::diagnostic::*;
use crate::utils::{get_arg_by_name_then_position, node_contains_comments};
use air_r_syntax::*;
use anyhow::Context;
use biome_rowan::AstNode;

pub struct Lengths;

/// ## What it does
///
/// Checks for usage of `length()` in several functions that apply it to each
/// element of a list, such as `lapply()`, `vapply()`, `purrr::map()`, etc.,
/// and replaces it with `lengths()`.
///
/// ## Why is this bad?
///
/// `lengths()` is faster and more memory-efficient than applying `length()`
/// on each element of the list.
///
/// ## Example
///
/// ```r
/// x <- list(a = 1, b = 2:3, c = 1:10)
/// sapply(x, length)
/// ```
///
/// Use instead:
/// ```r
/// x <- list(a = 1, b = 2:3, c = 1:10)
/// lengths(x)
/// ```
///
/// ## References
///
/// See `?lengths`
impl Violation for Lengths {
    fn name(&self) -> String {
        "lengths".to_string()
    }
    fn body(&self) -> String {
        "Using `length()` on each element of a list is inefficient.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `lengths()` instead.".to_string())
    }
}

pub fn lengths(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let RCallFields { function, arguments } = ast.as_fields();
    let function = function?;

    let funs_to_watch = ["sapply", "vapply", "map_dbl", "map_int"];
    if !funs_to_watch.contains(&function.into_syntax().text_trimmed().to_string().as_str()) {
        return Ok(None);
    }

    let arguments = arguments?.items();
    let arg_x = get_arg_by_name_then_position(&arguments, "x", 1);
    let arg_fun = get_arg_by_name_then_position(&arguments, "FUN", 2);

    if let Some(arg_fun) = arg_fun {
        if arg_fun
            .value()
            .context("Found named argument without any value")?
            .into_syntax()
            .text_trimmed()
            == "length"
        {
            let range = ast.syntax().text_trimmed_range();
            let diagnostic = Diagnostic::new(
                Lengths,
                range,
                Fix {
                    content: format!("lengths({})", arg_x.unwrap().into_syntax().text_trimmed()),
                    start: range.start().into(),
                    end: range.end().into(),
                    to_skip: node_contains_comments(ast.syntax()),
                },
            );
            return Ok(Some(diagnostic));
        }
    };

    Ok(None)
}
