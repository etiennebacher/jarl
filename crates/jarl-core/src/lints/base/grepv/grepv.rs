use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{Formals, drop_arg, get_arg, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

const FORMALS_GREP: Formals = &[
    "pattern",
    "x",
    "ignore.case",
    "perl",
    "value",
    "fixed",
    "useBytes",
    "invert",
];
pub struct Grepv;

/// Version added: 0.0.16
///
/// ## What it does
///
/// Checks for usage of `grep(..., value = TRUE)` and recommends using
/// `grepv()` instead (only if the R version used in the project is >= 4.5).
///
/// ## Why is this bad?
///
/// Starting from R 4.5, there is a function `grepv()` that is identical to
/// `grep()` except that it uses `value = TRUE` by default.
///
/// Using `grepv(...)` is therefore more readable than `grep(...)`.
///
/// ## Example
///
/// ```r
/// x <- c("hello", "hi", "howdie")
/// grep("i", x, value = TRUE)
/// ```
///
/// Use instead:
/// ```r
/// x <- c("hello", "hi", "howdie")
/// grepv("i", x)
/// ```
///
/// ## References
///
/// See `?grepv`
impl Violation for Grepv {
    fn rule(&self) -> Rule {
        Rule::Grepv
    }
    fn body(&self) -> String {
        "`grep(..., value = TRUE)` can be simplified.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `grepv(...)` instead.".to_string())
    }
}

pub fn grepv(ast: &RCall, fn_name: &str) -> anyhow::Result<Option<Diagnostic>> {
    if fn_name != "grep" {
        return Ok(None);
    }

    let arg_value_is_present = get_arg(ast, FORMALS_GREP, "value").is_some();

    if !arg_value_is_present {
        return Ok(None);
    }

    let other_args = drop_arg(ast, FORMALS_GREP, "value");

    let inner_content = match other_args {
        Some(x) => x
            .iter()
            .map(|x| x.syntax().text_trimmed().to_string())
            .collect::<Vec<_>>()
            .join(", "),
        None => "".to_string(),
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        Grepv,
        range,
        Fix::new(
            range,
            format!("grepv({inner_content})"),
            node_contains_comments(ast.syntax()),
        ),
    );

    Ok(Some(diagnostic))
}
