use crate::message::*;
use crate::utils::get_function_name;
use crate::utils::is_argument_present;
use air_r_syntax::*;
use anyhow::Result;
use biome_rowan::AstNode;
use biome_rowan::AstSeparatedList;
pub struct AnyDuplicated;

/// ## What it does
///
/// Checks for usage of `grep(..., value = TRUE)`.
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
impl Violation for AnyDuplicated {
    fn name(&self) -> String {
        "any-duplicated".to_string()
    }
    fn body(&self) -> String {
        "`any(duplicated(...))` is inefficient. Use `anyDuplicated(...) > 0` instead.".to_string()
    }
}

pub fn grepv(ast: &RCall) -> Result<Option<Diagnostic>> {
    let RCallFields { function, arguments } = ast.as_fields();

    let function = function?;
    let fn_name = get_function_name(function);

    if fn_name != "grep" {
        return Ok(None);
    }

    let items = arguments?.items();

    let arg_value_is_present = is_argument_present(&items, "value", 5);

    println!("arg_value_is_present: {}", arg_value_is_present);

    return Ok(None);

    // let named_args = items
    //     .iter()
    //     .filter(|x| x.clone().unwrap().name_clause().is_some())
    //     .collect::<Vec<_>>();

    // if !named_args.is_empty() {}

    // let unnamed_arg = &items
    //     .iter()
    //     .find(|x| x.clone().unwrap().name_clause().is_none());

    // // any(na.rm = TRUE/FALSE) and any() are valid
    // if unnamed_arg.is_none() {
    //     return Ok(None);
    // }

    // let value = unnamed_arg.unwrap()?.value();

    // if let Some(inner) = value
    //     && let Some(inner2) = inner.as_r_call()
    // {
    //     let RCallFields { function, arguments } = inner2.as_fields();

    //     let function = function?;
    //     let inner_fn_name = get_function_name(function);

    //     if inner_fn_name != "duplicated" {
    //         return Ok(None);
    //     }

    //     let inner_content = arguments?.items().into_syntax().text();
    //     let range = ast.clone().into_syntax().text_trimmed_range();
    //     let diagnostic = Diagnostic::new(
    //         AnyDuplicated,
    //         range,
    //         Fix {
    //             content: format!("anyDuplicated({inner_content}) > 0"),
    //             start: range.start().into(),
    //             end: range.end().into(),
    //         },
    //     );

    //     return Ok(Some(diagnostic));
    // }
    // return Ok(None);
}
