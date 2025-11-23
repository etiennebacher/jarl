use crate::diagnostic::*;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct Browser;

/// ## What it does
///
/// Checks for lingering presence of `browser()` which should not be present in
/// released code.
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

    if function.to_trimmed_text() != "browser" {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(Browser, range, Fix::empty());

    Ok(Some(diagnostic))
}
