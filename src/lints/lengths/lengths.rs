use crate::message::*;
use crate::traits::ArgumentListExt;
use air_r_syntax::*;
use anyhow::{Context, Result};
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
        "Use `lengths()` to find the length of each element in a list.".to_string()
    }
}

pub fn lengths(ast: &RCall) -> Result<Option<Diagnostic>> {
    let RCallFields { function, arguments } = ast.as_fields();
    let function = function?;

    let funs_to_watch = ["sapply", "vapply", "map_dbl", "map_int"];
    if !funs_to_watch.contains(&function.text().as_str()) {
        return Ok(None);
    }

    let arguments = arguments?.items();
    let arg_x = arguments.get_arg_by_name_then_position("x", 0);
    let arg_fun = arguments.get_arg_by_name_then_position("FUN", 1);

    if let Some(arg_fun) = arg_fun {
        if arg_fun
            .value()
            .context("Found named argument without any value")?
            .text()
            == "length"
        {
            let range = ast.clone().into_syntax().text_trimmed_range();
            let diagnostic = Diagnostic::new(
                Lengths,
                range,
                Fix {
                    content: format!("lengths({})", arg_x.unwrap().text()),
                    start: range.start().into(),
                    end: range.end().into(),
                },
            );
            return Ok(Some(diagnostic));
        }
    };

    Ok(None)
}
