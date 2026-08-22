use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{Formals, get_arg, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

const FORMALS_IFELSE: Formals = &["test", "yes", "no"];
/// Omits `ptype` and `size`, which follow `...`.
const FORMALS_IF_ELSE: Formals = &["condition", "true", "false", "missing"];
const FORMALS_FIFELSE: Formals = &["test", "yes", "no", "na"];

/// Version added: 0.4.0
///
/// ## What it does
///
/// This checks for cases of `ifelse()`, `dplyr::if_else()`, and
/// `data.table::fifelse()` where the output is always a boolean. In those cases,
/// using the condition directly is enough, the function call is redundant.
///
/// ## Why is this bad?
///
/// This rule looks for 4 cases:
///
/// - `ifelse(condition, TRUE, FALSE)`
/// - `ifelse(condition, FALSE, TRUE)`
/// - `ifelse(condition, TRUE, TRUE)`
/// - `ifelse(condition, FALSE, FALSE)`
///
/// The first two cases can be simplified to `condition` and `!condition`
/// respectively. The last two cases are very likely to be mistakes since the
/// output is always the same.
///
/// This rule has a safe fix and doesn't handle calls to `dplyr::if_else()` and
/// `data.table::fifelse()` when they have additional arguments.
///
/// ## Example
///
/// ```r
/// ifelse(x %in% letters, TRUE, FALSE)
/// dplyr::if_else(x > 1, FALSE, TRUE)
/// ```
///
/// Use instead:
/// ```r
/// x %in% letters
/// !(x > 1) # (or `x <= 1`)
/// ```
pub fn redundant_ifelse(ast: &RCall, fn_name: &str) -> anyhow::Result<Option<Diagnostic>> {
    if fn_name != "ifelse" && fn_name != "if_else" && fn_name != "fifelse" {
        return Ok(None);
    }

    let args = ast.arguments()?.items();
    let n_args = args.iter().collect::<Vec<_>>().len();

    // Don't want to handle additional args.
    if n_args != 3 {
        return Ok(None);
    }

    let formals = match fn_name {
        "if_else" => FORMALS_IF_ELSE,
        "fifelse" => FORMALS_FIFELSE,
        _ => FORMALS_IFELSE,
    };
    let (arg_cond, arg_true, arg_false) = (
        unwrap_or_return_none!(get_arg(ast, formals, formals[0])),
        unwrap_or_return_none!(get_arg(ast, formals, formals[1])),
        unwrap_or_return_none!(get_arg(ast, formals, formals[2])),
    );

    let arg_cond = unwrap_or_return_none!(arg_cond.value());
    let arg_true = unwrap_or_return_none!(arg_true.value());
    let arg_false = unwrap_or_return_none!(arg_false.value());

    let arg_true_is_true = arg_true.as_r_true_expression().is_some();
    let arg_true_is_false = arg_true.as_r_false_expression().is_some();
    let arg_false_is_true = arg_false.as_r_true_expression().is_some();
    let arg_false_is_false = arg_false.as_r_false_expression().is_some();

    if (!arg_true_is_true && !arg_true_is_false) || (!arg_false_is_true && !arg_false_is_false) {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();

    let (msg, suggestion, fix) = if arg_true_is_true && arg_false_is_false {
        (
            format!("This `{}()` is redundant.", fn_name),
            "Use `condition` directly.".to_string(),
            Fix::new(
                range,
                arg_cond.to_string(),
                node_contains_comments(ast.syntax()),
            ),
        )
    } else if arg_true_is_false && arg_false_is_true {
        (
            format!("This `{}()` is redundant.", fn_name),
            "Use `!condition` directly.".to_string(),
            Fix::new(
                range,
                format!("!({})", arg_cond),
                node_contains_comments(ast.syntax()),
            ),
        )
    } else if arg_true_is_true && arg_false_is_true {
        (
            format!("This `{}()` always evaluates to `TRUE`.", fn_name),
            "This is likely wrong.".to_string(),
            Fix::empty(),
        )
    } else if arg_true_is_false && arg_false_is_false {
        (
            format!("This `{}()` always evaluates to `FALSE`.", fn_name),
            "This is likely wrong.".to_string(),
            Fix::empty(),
        )
    } else {
        unreachable!()
    };

    let diagnostic = Diagnostic::new(
        ViolationData::new(Rule::RedundantIfelse, msg, Some(suggestion)),
        range,
        fix,
    );

    Ok(Some(diagnostic))
}
