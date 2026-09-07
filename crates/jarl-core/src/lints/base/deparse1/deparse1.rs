use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{get_arg_by_name, get_nested_functions_content, node_contains_comments};
use air_r_syntax::*;
use oak_core::syntax_ext::RStringValueExt;

pub struct Deparse1;

/// Version added: 0.6.1
///
/// ## What it does
///
/// Checks for usage of `paste(deparse(x), collapse = " ")`.
///
/// ## Why is this bad?
///
/// Since R 4.0.0, it is possible to do `paste(deparse(x), collapse = " ")`
/// with `deparse1(x)`, which is more efficient and easier to read.
///
/// This rule comes with an unsafe fix because `deparse1()`'s default
/// `width.cutoff` (500) differs from `deparse()`'s (60), so the two calls
/// can produce different output when `width.cutoff` isn't set explicitly.
/// This rule is only enabled if the project explicitly uses R >= 4.0.0 (or
/// if the argument `--min-r-version` is passed with a version >= 4.0.0).
///
/// ## Example
///
/// ```r
/// paste(deparse(args(library)), collapse = " ")
/// ```
///
/// Use instead:
/// ```r
/// deparse1(args(library))
/// ```
///
/// ## References
///
/// See `?deparse1`
impl Violation for Deparse1 {
    fn rule(&self) -> Rule {
        Rule::Deparse1
    }
    fn body(&self) -> String {
        "`paste(deparse(x), collapse = \" \")` is inefficient and can be hard to read.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `deparse1(x)` instead.".to_string())
    }
}

pub fn deparse1(ast: &RCall, fn_name: &str) -> anyhow::Result<Option<Diagnostic>> {
    let (inner_content, outer_syntax) = unwrap_or_return_none!(get_nested_functions_content(
        ast, fn_name, "paste", "deparse"
    )?);

    let collapse = unwrap_or_return_none!(get_arg_by_name(&ast.arguments()?.items(), "collapse"));
    let collapse_value = unwrap_or_return_none!(collapse.value());

    let collapse_r_value = unwrap_or_return_none!(collapse_value.as_any_r_value());
    let collapse_string = unwrap_or_return_none!(collapse_r_value.as_r_string_value());

    if collapse_string.string_text().as_deref() != Some(" ") {
        return Ok(None);
    }

    let range = outer_syntax.text_trimmed_range();
    let diagnostic = Diagnostic::new(
        Deparse1,
        range,
        Fix::new(
            range,
            format!("deparse1({inner_content})"),
            node_contains_comments(&outer_syntax),
        ),
    );

    Ok(Some(diagnostic))
}
