use crate::message::*;
use crate::utils::{get_arg_by_name, get_unnamed_args};
use air_r_syntax::*;
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

pub fn sort(ast: &RSubset) -> anyhow::Result<Option<Diagnostic>> {
    let RSubsetFields { function, arguments } = ast.as_fields();
    let function_outer = function?;
    let arguments = arguments?;

    let items: Vec<_> = arguments.items().into_iter().collect();

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

    let function = arg_value.function()?;
    let arg_inner = arg_value.arguments()?;

    if function.to_trimmed_text() != "order" {
        return Ok(None);
    }

    let args = arg_inner.items();
    let values = get_unnamed_args(&args);
    if values.len() != 1 {
        return Ok(None);
    }

    // Safety: we know that `values` contains a single element.
    let values = values.get(0).unwrap();
    if values.to_trimmed_text() != function_outer.to_trimmed_text() {
        return Ok(None);
    }

    // order() takes `...` so other args must be named.
    let na_last = get_arg_by_name(&args, "na.last");
    let decreasing = get_arg_by_name(&args, "decreasing");
    let method = get_arg_by_name(&args, "method");

    let mut additional_args = vec![];
    if let Some(na_last) = na_last {
        additional_args.push(na_last.to_trimmed_text());
    }
    if let Some(decreasing) = decreasing {
        additional_args.push(decreasing.to_trimmed_text());
    }
    if let Some(method) = method {
        additional_args.push(method.to_trimmed_text());
    }

    let additional_args = additional_args.join(", ");
    let fix = if additional_args.len() > 0 {
        format!(
            "sort({}, {})",
            function_outer.to_trimmed_text(),
            additional_args
        )
    } else {
        format!("sort({})", function_outer.to_trimmed_text())
    };
    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        Sort,
        range,
        Fix {
            content: fix,
            start: range.start().into(),
            end: range.end().into(),
        },
    );

    Ok(Some(diagnostic))
}
