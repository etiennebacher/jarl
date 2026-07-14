use crate::diagnostic::*;
use crate::utils::get_function_name;
use air_r_syntax::{RArgument, RCall};
use biome_rowan::AstNode;

pub struct StopifnotAll;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for direct calls to `all()` inside `stopifnot()`.
///
/// ## Why is this bad?
///
/// `stopifnot()` already checks `all()` of each argument internally. Passing
/// `all(x)` hides the original expression from `stopifnot()`, which results in
/// a less informative error message when the condition fails.
///
/// ## Example
///
/// ```r
/// stopifnot(all(x > 0))
/// ```
///
/// Use instead:
/// ```r
/// stopifnot(x > 0)
/// ```
///
/// ## References
///
/// See `?stopifnot`.
impl Violation for StopifnotAll {
    fn name(&self) -> String {
        "stopifnot_all".to_string()
    }

    fn body(&self) -> String {
        "`stopifnot(all(x))` produces a less informative error message.".to_string()
    }

    fn suggestion(&self) -> Option<String> {
        Some("Use `stopifnot(x)` instead.".to_string())
    }
}

pub fn stopifnot_all(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    if get_function_name(ast.function()?) != "all" {
        return Ok(None);
    }

    let Some(argument) = ast.syntax().parent().and_then(RArgument::cast) else {
        return Ok(None);
    };
    let outer_call = argument
        .syntax()
        .ancestors()
        .find_map(RCall::cast)
        .expect("an R argument must belong to a call");

    if get_function_name(outer_call.function()?) != "stopifnot" {
        return Ok(None);
    }

    Ok(Some(Diagnostic::new(
        StopifnotAll,
        ast.syntax().text_trimmed_range(),
        Fix::empty(),
    )))
}
