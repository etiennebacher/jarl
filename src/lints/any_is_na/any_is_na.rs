use crate::message::*;
use crate::utils::get_function_name;
use air_r_syntax::*;
use anyhow::Result;
use biome_rowan::AstNode;

pub struct AnyIsNa;

/// ## What it does
///
/// Checks for usage of `any(is.na(...))`.
///
/// ## Why is this bad?
///
/// `any(is.na(...))` is valid code but requires the evaluation of `is.na()` on
/// the entire input first.
///
/// There is a more efficient function in base R called `anyNA()` that is more
/// efficient, both in speed and memory used.
///
/// ## Example
///
/// ```r
/// x <- c(1:10000, NA)
/// any(is.na(x))
/// ```
///
/// Use instead:
/// ```r
/// x <- c(1:10000, NA)
/// anyNA(x)
/// ```
///
/// ## References
///
/// See `?anyNA`
impl Violation for AnyIsNa {
    fn name(&self) -> String {
        "any-na".to_string()
    }
    fn body(&self) -> String {
        "`any(is.na(...))` is inefficient. Use `anyNA(...)` instead.".to_string()
    }
}

pub fn any_is_na(ast: &RCall) -> Result<Option<Diagnostic>> {
    let RCallFields { function, arguments } = ast.as_fields();

    let function = function?;
    let outer_fn_name = get_function_name(function);

    if outer_fn_name != "any" {
        return Ok(None);
    }

    let items = arguments?.items();

    let unnamed_arg = items
        .into_iter()
        .find(|x| x.clone().unwrap().name_clause().is_none());

    // any(na.rm = TRUE/FALSE) and any() are valid
    if unnamed_arg.is_none() {
        return Ok(None);
    }

    let value = unnamed_arg.unwrap()?.value();

    if let Some(inner) = value
        && let Some(inner2) = inner.as_r_call()
    {
        let RCallFields { function, arguments } = inner2.as_fields();

        let function = function?;
        let inner_fn_name = get_function_name(function);

        if inner_fn_name != "is.na" {
            return Ok(None);
        }

        let inner_content = arguments?.items().into_syntax().text();
        let range = ast.clone().into_syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(
            AnyIsNa,
            range,
            Fix {
                content: format!("anyNA({inner_content})"),
                start: range.start().into(),
                end: range.end().into(),
            },
        );
        return Ok(Some(diagnostic));
    }

    Ok(None)
}
