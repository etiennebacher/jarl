use crate::message::*;
use crate::utils::get_nested_functions_content;
use air_r_syntax::*;
use anyhow::Result;
use biome_rowan::AstNode;

pub struct Sort;

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
impl Violation for Sort {
    fn name(&self) -> String {
        "sort".to_string()
    }
    fn body(&self) -> String {
        "`x[order(x)]` is inefficient. Use `sort(x)` instead.".to_string()
    }
}

pub fn sort(ast: &RSubset) -> Result<Option<Diagnostic>> {
    let RSubsetFields { function, arguments } = ast.as_fields();
    let function_outer = function?;
    let arguments = arguments?;

    let items: Vec<_> = arguments.items().into_iter().collect();

    println!("function_outer: {}", function_outer);
    println!("arguments: {}", arguments);

    // No lint for x[order(x), "bar"] or x[, order(x)].
    if items.len() != 1 {
        return Ok(None);
    }

    // Safety: we know that `items` contains a single element.
    let arg = items.get(0).unwrap().clone()?;

    // No lint for x[foo = order(x)].
    if arg.name_clause().is_some() {
        return Ok(None);
    }

    let Some(arg_value) = arg.value() else {
        return Ok(None);
    };

    let Some(arg_value) = arg_value.as_r_call() else {
        return Ok(None);
    };

    let RCallFields { function, arguments } = arg_value.as_fields();
    let function = function?;
    let arg_inner = arguments?;

    if function.to_trimmed_text() != "order" {
        return Ok(None);
    }

    // if let Some(inner_content) = inner_content {
    //     let range = ast.syntax().text_trimmed_range();
    //     let diagnostic = Diagnostic::new(
    //         Sort,
    //         range,
    //         Fix {
    //             content: format!("anyDuplicated({inner_content}) > 0"),
    //             start: range.start().into(),
    //             end: range.end().into(),
    //         },
    //     );

    //     return Ok(Some(diagnostic));
    // }

    Ok(None)
}
