use crate::diagnostic::*;
use crate::utils::{get_arg_by_name, get_unnamed_args};
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct Sort;

/// ## What it does
///
/// Checks for usage of `x[order(x, ...)]`.
///
/// ## Why is this bad?
///
/// It is better to use `sort(x, ...)`, which is more readable than
/// `x[order(x, ...)]` and more efficient.
///
/// ## Example
///
/// ```r
/// x <- c(3, 2, 5, 1, 5, 6)
/// x[order(x)]
/// x[order(x, na.last = TRUE)]
/// x[order(x, decreasing = TRUE)]
/// ```
///
/// Use instead:
/// ```r
/// x <- c(3, 2, 5, 1, 5, 6)
/// sort(x)
/// sort(x, na.last = TRUE)
/// sort(x, decreasing = TRUE)
/// ```
///
/// ## References
///
/// See `?sort`
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

    let inside_brackets: Vec<_> = arguments.items().into_iter().collect();

    // No lint for x[order(x), "bar"] or x[, order(x)].
    if inside_brackets.len() != 1 {
        return Ok(None);
    }

    // Safety: we know that `inside_brackets` contains a single element.
    let arg = inside_brackets.first().unwrap().clone()?;

    // No lint for x[foo = order(x)].
    if arg.name_clause().is_some() {
        return Ok(None);
    }

    let Some(arg_value) = arg.value() else {
        return Ok(None);
    };

    // Ensure we have something like `x[order(...)]`.
    let Some(arg_value) = arg_value.as_r_call() else {
        return Ok(None);
    };
    let function = arg_value.function()?;
    let arg_inner = arg_value.arguments()?;
    if function.to_trimmed_text() != "order" {
        return Ok(None);
    }

    // Get the main argument of `order()`.
    let args = arg_inner.items();
    let values = get_unnamed_args(&args);
    if values.len() != 1 {
        return Ok(None);
    }
    // Safety: we know that `values` contains a single element.
    let values = values.first().unwrap();
    if values.to_trimmed_text() != function_outer.to_trimmed_text() {
        return Ok(None);
    }

    // order() takes `...` so other args must be named.
    let na_last = get_arg_by_name(&args, "na.last");
    let decreasing = get_arg_by_name(&args, "decreasing");
    let method = get_arg_by_name(&args, "method");

    // Prepare text of other args to include in the fix.
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
    let fix = if additional_args.is_empty() {
        format!("sort({})", function_outer.to_trimmed_text())
    } else {
        format!(
            "sort({}, {})",
            function_outer.to_trimmed_text(),
            additional_args
        )
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
