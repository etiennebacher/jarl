use crate::diagnostic::*;
use crate::utils::drop_arg_by_name_or_position;
use crate::utils::get_function_name;
use crate::utils::is_argument_present;
use air_r_syntax::*;
use biome_rowan::AstNode;
pub struct Grepv;

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
    fn name(&self) -> String {
        "grepv".to_string()
    }
    fn body(&self) -> String {
        "Use `grepv(...)` instead of `grep(..., value = TRUE)`.".to_string()
    }
}

pub fn grepv(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let RCallFields { function, arguments } = ast.as_fields();

    let function = function?;
    let fn_name = get_function_name(function);

    if fn_name != "grep" {
        return Ok(None);
    }

    let items = arguments?.items();

    let arg_value_is_present = is_argument_present(&items, "value", 5);

    if !arg_value_is_present {
        return Ok(None);
    }

    let other_args = drop_arg_by_name_or_position(&items, "value", 5);

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
        Fix {
            content: format!("grepv({inner_content})"),
            start: range.start().into(),
            end: range.end().into(),
        },
    );

    Ok(Some(diagnostic))
}
