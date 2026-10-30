use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{Formals, get_arg, get_arg_by_name, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;
use biome_rowan::AstSeparatedList;

const FORMALS_APPLY: Formals = &["X", "MARGIN", "FUN"];

/// <!-- docs: start -->
/// Version added: 0.8.0
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
/// The automated fix is marked unsafe because the result of
/// `do.call(paste, c(x, sep = ...))` depends on the type and content of `x`.
/// It can differ from the original call when:
///
/// - `x` is a matrix: `c(x, sep = ...)` is not a list, so `do.call()` errors.
/// - `x` is a `data.frame` mixing character and non-character columns:
///   `apply()` converts it with `as.matrix()`, which pads numbers and logicals
///   to a common width (e.g. `" 1_x"` instead of `"1_x"`).
/// - `x` has a column named `sep`, `collapse`, or `recycle0`: it is matched to
///   the argument of `paste()` with the same name, which either errors or
///   changes the output.
/// - `x` has non-automatic row names: `apply()` uses them to name the output,
///   `do.call(paste, ...)` does not.
/// - `collapse` is a variable that evaluates to `NULL`: `apply()` doesn't
///   collapse rows, while `c()` drops `sep = NULL` so `paste()` uses its
///   default `sep = " "`.
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
/// <!-- docs: end -->
pub fn apply_paste(
    ast: &RCall,
    fn_name: &str,
    ns_prefix: Option<&str>,
) -> anyhow::Result<Option<Diagnostic>> {
    if fn_name != "apply" || ns_prefix.is_some_and(|ns| ns != "base::") {
        return Ok(None);
    }

    let args = ast.arguments()?.items();

    // Only handle the clean `apply(X, MARGIN, paste, collapse = ...)` form.
    // Any extra argument would also be forwarded to `paste()` and can't be
    // translated to `do.call()` unambiguously, so we bail in that case.
    if args.iter().count() != 4 {
        return Ok(None);
    }

    let x = get_arg(ast, FORMALS_APPLY, "X");
    let margin = get_arg(ast, FORMALS_APPLY, "MARGIN");
    let fun = get_arg(ast, FORMALS_APPLY, "FUN");
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
    // `collapse = NULL` doesn't collapse rows, so there's nothing to rewrite.
    if collapse_value.as_r_null_expression().is_some() {
        return Ok(None);
    }
    let collapse = collapse_value.to_trimmed_string();

    let range = ast.syntax().text_trimmed_range();
    let fix = format!("do.call(paste, c({x}, sep = {collapse}))");

    let diagnostic = Diagnostic::new(
        ViolationData::new(
            Rule::ApplyPaste,
            "`apply()` with `paste()` is inefficient.".to_string(),
            Some(
                "If `x` is a data.frame, use `do.call(paste, c(x, sep = ...))` instead."
                    .to_string(),
            ),
        ),
        range,
        Fix::new(range, fix, node_contains_comments(ast.syntax())),
    );

    Ok(Some(diagnostic))
}
