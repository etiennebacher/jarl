use crate::message::*;
use crate::utils::get_function_name;
use air_r_syntax::*;
use anyhow::Result;
use biome_rowan::AstNode;

pub struct WhichGrepl;

/// ## What it does
///
/// Checks for usage of `which(grepl(...))` and replaces it with `grep(...)`.
///
/// ## Why is this bad?
///
/// `which(grepl(...))` is harder to read and is less efficient than `grep()`
/// since it requires two passes on the vector.
///
/// ## Example
///
/// ```r
/// x <- c("hello", "there")
/// which(grepl("hell", x))
/// which(grepl("foo", x))
/// ```
///
/// Use instead:
/// ```r
/// x <- c("hello", "there")
/// grep("hell", x)
/// grep("foo", x)
/// ```
///
/// ## References
///
/// See `?grep`
impl Violation for WhichGrepl {
    fn name(&self) -> String {
        "which_grepl".to_string()
    }
    fn body(&self) -> String {
        "`grep(pattern, x)` is better than `which(grepl(pattern, x))`.".to_string()
    }
}

pub fn which_grepl(ast: &RCall) -> Result<Option<Diagnostic>> {
    let RCallFields { function, arguments } = ast.as_fields();

    let function = function?;
    let outer_fn_name = get_function_name(function);

    if outer_fn_name != "which" {
        return Ok(None);
    }

    let items = arguments?.items();

    let unnamed_arg = items
        .into_iter()
        .find(|x| x.clone().unwrap().name_clause().is_none());

    let value = unnamed_arg.unwrap()?.value();

    if let Some(inner) = value
        && let Some(inner2) = inner.as_r_call()
    {
        let RCallFields { function, arguments } = inner2.as_fields();

        let function = function?;
        let inner_fn_name = get_function_name(function);

        if inner_fn_name != "grepl" {
            return Ok(None);
        }

        let inner_content = arguments?.items().into_syntax().text();
        let range = ast.clone().into_syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(
            WhichGrepl,
            range,
            Fix {
                content: format!("grep({inner_content})"),
                start: range.start().into(),
                end: range.end().into(),
            },
        );
        return Ok(Some(diagnostic));
    }
    Ok(None)
}
