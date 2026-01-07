use crate::diagnostic::*;
use crate::utils::{get_arg_by_name_then_position, get_function_name, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

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
pub fn redundant_ifelse(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let fn_name = get_function_name(function);

    if fn_name != "ifelse" && fn_name != "if_else" && fn_name != "fifelse" {
        return Ok(None);
    }

    let args = ast.arguments()?.items();
    let n_args = args.iter().collect::<Vec<_>>().len();

    // Don't want to handle additional args.
    if n_args != 3 {
        return Ok(None);
    }

    let (arg_cond, arg_true, arg_false) = match fn_name.as_str() {
        "ifelse" => (
            unwrap_or_return_none!(get_arg_by_name_then_position(&args, "test", 1)),
            unwrap_or_return_none!(get_arg_by_name_then_position(&args, "yes", 2)),
            unwrap_or_return_none!(get_arg_by_name_then_position(&args, "no", 3)),
        ),
        "if_else" => (
            unwrap_or_return_none!(get_arg_by_name_then_position(&args, "condition", 1)),
            unwrap_or_return_none!(get_arg_by_name_then_position(&args, "true", 2)),
            unwrap_or_return_none!(get_arg_by_name_then_position(&args, "false", 3)),
        ),
        "fifelse" => (
            unwrap_or_return_none!(get_arg_by_name_then_position(&args, "test", 1)),
            unwrap_or_return_none!(get_arg_by_name_then_position(&args, "yes", 2)),
            unwrap_or_return_none!(get_arg_by_name_then_position(&args, "no", 3)),
        ),
        _ => unreachable!(),
    };

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
            Fix {
                content: arg_cond.to_string(),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        )
    } else if arg_true_is_false && arg_false_is_true {
        (
            format!("This `{}()` is redundant.", fn_name),
            "Use `!condition` directly.".to_string(),
            Fix {
                content: format!("!({})", arg_cond.to_string()),

                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
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
        ViolationData::new("redundant_ifelse".to_string(), msg, Some(suggestion)),
        range,
        fix,
    );

    Ok(Some(diagnostic))
}
