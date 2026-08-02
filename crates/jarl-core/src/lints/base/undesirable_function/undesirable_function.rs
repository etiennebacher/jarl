use crate::checker::Checker;
use crate::diagnostic::*;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct UndesirableFunction {
    pub fn_name: String,
}

/// Version added: 0.5.0
///
/// ## What it does
///
/// Checks for calls to functions listed as undesirable.
///
/// ## Why is this bad?
///
/// Some functions should not appear in production code. For example,
/// `browser()` is a debugging tool that interrupts execution, and should be
/// removed before committing.
///
/// ## Configuration
///
/// By default, only `browser` is flagged. You can customise the list in
/// `jarl.toml`:
///
/// ```toml
/// [lint.undesirable_function]
/// # Replace the default list entirely:
/// functions = ["browser", "debug"]
///
/// # Or add to the defaults:
/// extend-functions = ["debug"]
/// ```
///
/// ## Example
///
/// ```r
/// do_something <- function(abc = 1) {
///    xyz <- abc + 1
///    browser()      # flagged by default
///    xyz
/// }
/// ```
impl Violation for UndesirableFunction {
    fn name(&self) -> String {
        "undesirable_function".to_string()
    }
    fn body(&self) -> String {
        format!("`{}()` is listed as an undesirable function.", self.fn_name)
    }
}

pub fn undesirable_function(
    ast: &RCall,
    fn_name: &str,
    checker: &Checker,
) -> anyhow::Result<Option<Diagnostic>> {
    if !checker
        .rule_options
        .undesirable_function
        .functions
        .contains(fn_name)
    {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        UndesirableFunction { fn_name: fn_name.to_string() },
        range,
        Fix::empty(),
    );

    Ok(Some(diagnostic))
}
