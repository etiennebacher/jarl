use crate::diagnostic::*;
use crate::utils::{get_arg_by_name, get_arg_by_name_then_position, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;
use biome_rowan::AstSeparatedList;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for usage of `apply(x, 1, paste, collapse = ...)` to paste together
/// the columns of each row.
///
/// ## Why is this bad?
///
/// `apply()` coerces its input to a matrix and calls `paste()` once per row,
/// which is slow. Since `paste()` is vectorized, the same result can be
/// obtained in a single call with `do.call(paste, c(x, sep = ...))`, which is
/// both faster and clearer.
///
/// The automated fix is marked unsafe because `do.call(paste, c(x, sep = ...))`
/// only reproduces the original result when `x` is a `data.frame` (a list of
/// columns). For a plain matrix, `c(x, sep = ...)` flattens all elements
/// instead of keeping one entry per column, so the fix would change the result.
///
/// ## Example
///
/// ```r
/// apply(df[, c("x", "y")], 1, paste, collapse = "_")
/// ```
///
/// Use instead:
/// ```r
/// do.call(paste, c(df[, c("x", "y")], sep = "_"))
/// ```
///
/// ## References
///
/// See `?do.call` and `?paste`
pub fn apply_paste(ast: &RCall, fn_name: &str) -> anyhow::Result<Option<Diagnostic>> {
    if fn_name != "apply" {
        return Ok(None);
    }

    let args = ast.arguments()?.items();

    // Only handle the clean `apply(X, MARGIN, paste, collapse = ...)` form.
    // Any extra argument would also be forwarded to `paste()` and can't be
    // translated to `do.call()` unambiguously, so we bail in that case.
    if args.iter().count() != 4 {
        return Ok(None);
    }

    let x = get_arg_by_name_then_position(&args, "X", 1);
    let margin = get_arg_by_name_then_position(&args, "MARGIN", 2);
    let fun = get_arg_by_name_then_position(&args, "FUN", 3);
    let collapse = unwrap_or_return_none!(get_arg_by_name(&args, "collapse"));

    let fun_value = unwrap_or_return_none!(fun.and_then(|arg| arg.value()));
    let fun = fun_value.to_trimmed_string();
    if fun != "paste" && fun != "base::paste" {
        return Ok(None);
    }

    // The `do.call(paste, c(x, sep = ...))` rewrite only reproduces the row-wise
    // paste when `MARGIN` is 1.
    let margin_value = unwrap_or_return_none!(margin.and_then(|arg| arg.value()));
    let margin_text = margin_value.to_trimmed_string();
    if margin_text != "1" && margin_text != "1L" {
        return Ok(None);
    }

    let x_value = unwrap_or_return_none!(x.and_then(|arg| arg.value()));
    let x = x_value.to_trimmed_string();

    let collapse_value = unwrap_or_return_none!(collapse.value());
    let collapse = collapse_value.to_trimmed_string();

    let range = ast.syntax().text_trimmed_range();
    let fix = format!("do.call(paste, c({x}, sep = {collapse}))");

    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "apply_paste".to_string(),
            "`apply()` with `paste()` is inefficient.".to_string(),
            Some("Use `do.call(paste, c(x, sep = ...))` instead.".to_string()),
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
