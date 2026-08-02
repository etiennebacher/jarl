use crate::check::Checker;
use crate::diagnostic::*;
use air_r_syntax::*;
use biome_rowan::AstNode;
use biome_rowan::AstSeparatedList;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for empty arguments in function calls, e.g. `paste("a", , "b")`.
///
/// ## Why is this bad?
///
/// An empty argument left between commas is often a typo: a value was either
/// deleted by mistake or never filled in. Depending on the function it can lead
/// to an error or to a silently wrong result.
///
/// Several functions (e.g. `mutate()` in the `tidyverse` ecosystem) allow
/// trailing commas. Those are ignored by default but you can also tweak this
/// list of ignored functions in `jarl.toml`:
///
/// ```ignore
/// ...
/// [lint.missing_argument]
/// extend-skipped-functions = ["my_function"]
/// ```
///
/// See the [rule-specific arguments](https://jarl.etiennebacher.com/reference/config-file#rule-specific-arguments)
/// for more information.
///
/// This rule has no automatic fix.
///
/// ## Example
///
/// ```r
/// paste("a", , "b")
/// mean(x, )
/// ```
///
/// Use instead:
/// ```r
/// paste("a", "b")
/// mean(x)
/// ```
/// (or add additional arguments).
pub fn missing_argument(
    ast: &RCall,
    fn_name: &str,
    checker: &Checker,
) -> anyhow::Result<Option<Diagnostic>> {
    if checker
        .rule_options
        .missing_argument
        .skipped_functions
        .contains(fn_name)
    {
        return Ok(None);
    }

    let args = ast.arguments()?;
    let missing_arg_idx = args
        .items()
        .iter()
        .enumerate()
        .filter(|(_, x)| x.clone().ok().unwrap().is_hole())
        .map(|(index, _)| (index + 1).to_string())
        .collect::<Vec<_>>();

    if missing_arg_idx.is_empty() {
        return Ok(None);
    }

    let msg = if missing_arg_idx.len() == 1 {
        format!("Argument {} is empty.", missing_arg_idx.first().unwrap())
    } else if missing_arg_idx.len() == 2 {
        let (last, rest) = missing_arg_idx.split_last().unwrap();
        format!("Arguments {} and {} are empty.", rest.join(", "), last)
    } else {
        let (last, rest) = missing_arg_idx.split_last().unwrap();
        format!("Arguments {}, and {} are empty.", rest.join(", "), last)
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "missing_argument".to_string(),
            msg.to_string(),
            Some("Consider removing or filling them.".to_string()),
        ),
        range,
        Fix::empty(),
    );

    Ok(Some(diagnostic))
}
