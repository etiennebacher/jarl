use crate::diagnostic::*;
use crate::utils::get_function_name;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct Browser;

/// ## What it does
///
/// Checks for lingering presence of `browser()` which should not be present in
/// released code.
///
/// **This rule is deprecated and will be removed in a future version. Use the
/// rule [`undesirable_function`](https://jarl.etiennebacher.com/rules/undesirable_function)
/// and configure it to report calls to `browser()` instead.**
///
/// ## Why is this bad?
///
/// `browser()` interrupts the execution of an expression and allows the inspection
/// of the environment where `browser()` was called from. This is helpful while
/// developing a function, but is not expected to be called by the user. Does not
/// remove the call as it does not have a suitable replacement.
///
/// ## Example
///
/// ```r
/// do_something <- function(abc = 1) {
///    xyz <- abc + 1
///    browser()      # This should be removed.
///    xyz
/// }
///
/// ```
///
/// ## References
///
/// See `?browser`
impl Violation for Browser {
    fn name(&self) -> String {
        "browser".to_string()
    }
    fn body(&self) -> String {
        "Calls to `browser()` should be removed.".to_string()
    }
}

pub fn browser(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let fn_name = get_function_name(function);

    if fn_name != "browser" {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(Browser, range, Fix::empty());

    Ok(Some(diagnostic))
}
