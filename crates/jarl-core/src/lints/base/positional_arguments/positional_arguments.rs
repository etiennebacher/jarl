use crate::checker::Checker;
use crate::diagnostic::*;
use crate::utils::get_unnamed_args;
use air_r_syntax::*;
use biome_rowan::AstNode;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Reports function calls that use more than a configurable number of positional
/// (unnamed) arguments.
///
/// ## Why is this bad?
///
/// Relying on argument position forces the reader to remember the function's
/// signature to understand what each value means, and makes calls fragile to
/// changes in the argument order. Naming the arguments documents intent at the
/// call site and is more robust.
///
/// The maximum number of allowed positional arguments is 2 by default and can
/// be customized with the `max-positional-args` option in `jarl.toml` (see [rule-specific arguments](https://jarl.etiennebacher.com/reference/config-file#rule-specific-arguments)).
///
/// ## Example
///
/// ```r
/// grepl("a", x, TRUE)
/// ```
///
/// Use instead:
///
/// ```r
/// grepl("a", x, ignore.case = TRUE)
/// ```
pub fn positional_arguments(ast: &RCall, checker: &Checker) -> anyhow::Result<Option<Diagnostic>> {
    let max_positional_args = checker
        .rule_options
        .positional_arguments
        .max_positional_args;

    let args = ast.arguments()?.items();
    let n_positional = get_unnamed_args(&args).len();

    if n_positional <= max_positional_args {
        return Ok(None);
    }

    let plural = if n_positional > 1 {
        "arguments"
    } else {
        "argument"
    };
    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "positional_arguments".to_string(),
            format!(
                "Calling a function with {n_positional} positional {plural} can be hard to read and is prone to mistakes."
            ),
            Some("Name the arguments to clarify what each value refers to.".to_string()),
        ),
        range,
        Fix::empty(),
    );

    Ok(Some(diagnostic))
}
