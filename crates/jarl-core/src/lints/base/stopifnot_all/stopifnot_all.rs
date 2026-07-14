use crate::diagnostic::*;
use crate::utils::{get_arg_by_name, get_function_name, node_contains_comments};
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
/// `stopifnot()` already checks that all values of each argument are `TRUE`.
/// Wrapping an argument in `all()` is therefore unnecessary.
/// Calls that explicitly set `na.rm` are not reported because removing `all()`
/// would not preserve their missing-value handling.
///
/// This rule has an automated fix that is marked unsafe and therefore requires
/// passing `--unsafe-fixes`. This is because `all()` coerces its arguments to
/// logical vectors, so removing it can change runtime behavior.
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
        "`stopifnot(all(...))` contains an unnecessary call to `all()`.".to_string()
    }

    fn suggestion(&self) -> Option<String> {
        Some("Use `stopifnot(...)` instead.".to_string())
    }
}

pub fn stopifnot_all(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    // Start from `all()` because it can appear in any argument of `stopifnot()`,
    // then verify that the containing call is `stopifnot()`.

    if get_function_name(ast.function()?) != "all" {
        return Ok(None);
    }

    let arguments = ast.arguments()?.items();
    if get_arg_by_name(&arguments, "na.rm").is_some() {
        return Ok(None);
    }

    let argument = unwrap_or_return_none!(ast.syntax().parent().and_then(RArgument::cast));
    let outer_call = argument
        .syntax()
        .ancestors()
        .find_map(RCall::cast)
        .ok_or_else(|| anyhow::anyhow!("an R argument must belong to a call"))?;

    if get_function_name(outer_call.function()?) != "stopifnot" {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();
    Ok(Some(Diagnostic::new(
        StopifnotAll,
        range,
        Fix {
            content: arguments.into_syntax().to_string(),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    )))
}
