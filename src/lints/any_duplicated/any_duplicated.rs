use crate::message::*;
use crate::utils::get_function_name;
use air_r_syntax::*;
use anyhow::Result;
use biome_rowan::AstNode;

pub struct AnyDuplicated;

/// ## What it does
///
/// Checks for usage of `any(duplicated(...))`.
///
/// ## Why is this bad?
///
/// `any(duplicated(...))` is valid code but requires the evaluation of
/// `duplicated()` on the entire input first.
///
/// There is a more efficient function in base R called `anyDuplicated()` that
/// is more efficient, both in speed and memory used. `anyDuplicated()` returns
/// the index of the first duplicated value, or 0 if there is none.
///
/// Therefore, we can replace `any(duplicated(...))` by `anyDuplicated(...) > 0`.
///
/// ## Example
///
/// ```r
/// x <- c(1:10000, 1, NA)
/// any(duplicated(x))
/// ```
///
/// Use instead:
/// ```r
/// x <- c(1:10000, 1, NA)
/// anyDuplicated(x) > 0
/// ```
///
/// ## References
///
/// See `?anyDuplicated`
impl Violation for AnyDuplicated {
    fn name(&self) -> String {
        "any-duplicated".to_string()
    }
    fn body(&self) -> String {
        "`any(duplicated(...))` is inefficient. Use `anyDuplicated(...) > 0` instead.".to_string()
    }
}

pub fn any_duplicated(ast: &RCall) -> Result<Option<Diagnostic>> {
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

        if inner_fn_name != "duplicated" {
            return Ok(None);
        }

        let inner_content = arguments?.items().into_syntax().text();
        let range = ast.clone().into_syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(
            AnyDuplicated,
            range,
            Fix {
                content: format!("anyDuplicated({inner_content}) > 0"),
                start: range.start().into(),
                end: range.end().into(),
            },
        );

        return Ok(Some(diagnostic));
    }
    return Ok(None);
}
