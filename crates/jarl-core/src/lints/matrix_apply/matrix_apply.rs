use crate::diagnostic::*;
use crate::utils::{get_arg_by_name, get_arg_by_name_then_position, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;
use biome_rowan::AstSeparatedList;

/// ## What it does
///
/// Checks for usage of `apply(x, 1/2, mean/sum)`.
///
/// ## Why is this bad?
///
/// `apply()` with `FUN = sum` or `FUN = mean` are inefficient when `MARGIN` is
/// 1 or 2. `colSums()`, `rowSums()`, `colMeans()`, `rowMeans()` are both easier
/// to read and much more efficient.
///
/// This rule provides an automated fix, except when extra arguments (outside
/// of `na.rm`) are provided. In other words, this would be marked as lint and
/// could be automatically replaced:
/// ```r
/// dat <- data.frame(x = 1:3, y = 4:6)
/// apply(dat, 1, mean, na.rm = TRUE)
/// ```
/// but this wouldn't:
/// ```r
/// dat <- data.frame(x = 1:3, y = 4:6)
/// apply(dat, 1, mean, trim = 0.2)
/// ```
///
/// ## Example
///
/// ```r
/// dat <- data.frame(x = 1:3, y = 4:6)
/// apply(dat, 1, sum)
/// apply(dat, 2, sum)
/// apply(dat, 1, mean)
/// apply(dat, 2, mean)
/// apply(dat, 2, mean, na.rm = TRUE)
/// ```
///
/// Use instead:
/// ```r
/// dat <- data.frame(x = 1:3, y = 4:6)
/// rowSums(dat)
/// colSums(dat)
/// rowMeans(dat)
/// colMeans(dat)
/// colMeans(dat, na.rm = TRUE)
/// ```
///
/// ## References
///
/// See `?colSums`
pub fn matrix_apply(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    if function.to_trimmed_string() != "apply" {
        return Ok(None);
    }

    let args = ast.arguments()?.items();
    let x = get_arg_by_name_then_position(&args, "X", 1);
    let margin = get_arg_by_name_then_position(&args, "MARGIN", 2);
    let fun = get_arg_by_name_then_position(&args, "FUN", 3);

    // We allow having `na.rm` as additional argument but it must be named anyway.
    // If it is present and we still have more than 4 args, it means that there
    // are extra args that we don't know how to handle so we just exit early.
    let na_rm = get_arg_by_name(&args, "na.rm");
    let is_na_rm_present = na_rm.is_some();
    if (is_na_rm_present && args.iter().count() > 4)
        || (!is_na_rm_present && args.iter().count() > 3)
    {
        return Ok(None);
    }

    let Some(x_value) = x.and_then(|arg| arg.value()) else {
        return Ok(None);
    };
    let x = x_value.to_trimmed_string();

    let Some(fun_value) = fun.and_then(|arg| arg.value()) else {
        return Ok(None);
    };
    let fun = fun_value.to_trimmed_string();

    if fun != "mean" && fun != "sum" {
        return Ok(None);
    }

    // MARGIN could be c(1, 2), in which case we don't know what to do.
    let Some(margin_value) = margin.and_then(|arg| arg.value()) else {
        return Ok(None);
    };

    let margin_text = margin_value.to_trimmed_string();
    let margin = if margin_text == "1" || margin_text == "1L" {
        "1"
    } else if margin_text == "2" || margin_text == "2L" {
        "2"
    } else {
        return Ok(None);
    };

    let fun = fun.as_str();
    let range = ast.syntax().text_trimmed_range();
    let (msg, suggestion) = match (fun, margin) {
        ("mean", "1") => (
            "`apply(x, 1, mean)` is inefficient.",
            "Use `rowMeans(x)` instead.",
        ),
        ("mean", "2") => (
            "`apply(x, 2, mean)` is inefficient.",
            "Use `colMeans(x)` instead.",
        ),
        ("sum", "1") => (
            "`apply(x, 1, sum)` is inefficient.",
            "Use `rowSums(x)` instead.",
        ),
        ("sum", "2") => (
            "`apply(x, 2, sum)` is inefficient.",
            "Use `colSums(x)` instead.",
        ),
        _ => unreachable!(),
    };

    let fix_na_rm = if is_na_rm_present {
        [", ", &na_rm.unwrap().to_trimmed_string()].join("")
    } else {
        "".to_string()
    };

    let fix = match (fun, margin) {
        ("mean", "1") => format!("rowMeans({x}{fix_na_rm})"),
        ("mean", "2") => format!("colMeans({x}{fix_na_rm})"),
        ("sum", "1") => format!("rowSums({x}{fix_na_rm})"),
        ("sum", "2") => format!("colSums({x}{fix_na_rm})"),
        _ => unreachable!(),
    };

    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "matrix_apply".to_string(),
            msg.to_string(),
            Some(suggestion.to_string()),
        ),
        range,
        Fix {
            content: fix,
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
