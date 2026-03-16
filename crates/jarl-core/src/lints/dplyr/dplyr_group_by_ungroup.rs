use crate::check::Checker;
use crate::diagnostic::*;
use crate::utils::{get_function_name, get_function_namespace_prefix, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::{AstNode, TextRange};

// List of dplyr verbs that support per-operation grouping.
// Verbs use `.by` except `slice_*()` which use `by`.
const VERBS_WITH_BY: &[&str] = &[
    "summarize",
    "summarise",
    "mutate",
    "filter",
    "reframe",
    "slice",
    "slice_head",
    "slice_tail",
    "slice_min",
    "slice_max",
    "slice_sample",
];

// Return the name of the grouping argument for a given verb.
fn by_arg_name(verb: &str) -> &'static str {
    // slice() has ".by", its variants have "by"
    if verb.starts_with("slice_") {
        "by"
    } else {
        ".by"
    }
}

/// ## What it does
///
/// Checks for `group_by() |> verb() |> ungroup()` patterns that can be
/// simplified using the `.by` or `by` argument.
///
/// ## Why is this bad?
///
/// Since `dplyr` 1.1.0, verbs like `summarize()`, `mutate()`, `filter()`,
/// `reframe()`, and the `slice_*()` family support a `.by` or `by` argument.
/// Using `.by` / `by` is shorter and does not require a subsequent `ungroup()`
/// call.
///
/// ## Example
///
/// ```r
/// x |>
///   group_by(grp) |>
///   slice_head(mean_val = mean(val)) |>
///   ungroup()
///
/// x |>
///   group_by(grp1, grp2) |>
///   summarize(mean_val = mean(val)) |>
///   ungroup()
/// ```
///
/// Use instead:
/// ```r
/// x |>
///   slice_head(mean_val = mean(val), by = grp)
///
/// x |>
///   summarize(mean_val = mean(val), .by = c(grp1, grp2))
/// ```
///
/// ## References
///
/// See the `.by` argument in `?dplyr::summarize`.
pub fn dplyr_group_by_ungroup(
    ast: &RCall,
    checker: &Checker,
) -> anyhow::Result<Option<Diagnostic>> {
    let fn_name = get_function_name(ast.function()?);
    let fn_ns = get_function_namespace_prefix(ast.function()?);

    // Only trigger on `ungroup()` or `dplyr::ungroup()`
    if fn_name != "ungroup" {
        return Ok(None);
    }
    if let Some(ref ns) = fn_ns
        && ns != "dplyr::"
    {
        return Ok(None);
    }

    // `.by` was introduced in dplyr 1.1.0. Skip if the installed
    // version is older.
    if let Some(version) = checker.package_version("dplyr")
        && version < (1, 1, 0)
    {
        return Ok(None);
    }

    // `ungroup()` must have no unnamed arguments (piped input only)
    let ungroup_args = ast.arguments()?;
    let ungroup_has_unnamed_args = ungroup_args
        .items()
        .into_iter()
        .any(|x| x.is_ok_and(|a| a.name_clause().is_none()));
    if ungroup_has_unnamed_args {
        return Ok(None);
    }

    // `ungroup()` must be on the right side of a pipe
    let parent_syntax = unwrap_or_return_none!(ast.syntax().parent());
    let pipe_expr = unwrap_or_return_none!(RBinaryExpression::cast(parent_syntax));

    let RBinaryExpressionFields { left, operator, .. } = pipe_expr.as_fields();
    if !is_pipe(&operator?) {
        return Ok(None);
    }
    let left = left?;

    // The left side must be a pipe where the right side is a supported verb:
    // `... |> verb(...) |> ungroup()`
    let verb_pipe = unwrap_or_return_none!(left.as_r_binary_expression());
    let RBinaryExpressionFields {
        left: verb_left,
        operator: verb_operator,
        right: verb_right,
    } = verb_pipe.as_fields();
    if !is_pipe(&verb_operator?) {
        return Ok(None);
    }
    let verb_call = unwrap_or_return_none!(verb_right?.as_r_call().cloned());
    let verb_name = get_function_name(verb_call.function()?);
    let verb_ns = get_function_namespace_prefix(verb_call.function()?);

    if !VERBS_WITH_BY.contains(&verb_name.as_str()) {
        return Ok(None);
    }
    if let Some(ref ns) = verb_ns
        && ns != "dplyr::"
    {
        return Ok(None);
    }

    // The verb must not already have a `by`/`.by` argument
    let by_arg = by_arg_name(&verb_name);
    let verb_args = verb_call.arguments()?;
    let has_by_arg = verb_args.items().into_iter().any(|x| {
        x.is_ok_and(|a| {
            a.name_clause()
                .is_some_and(|nc| nc.name().is_ok_and(|n| n.to_trimmed_string() == by_arg))
        })
    });
    if has_by_arg {
        return Ok(None);
    }

    // The left side of the verb pipe must end with `group_by(...)`:
    // either `... |> group_by(...)` or just `group_by(data, ...)`
    let verb_left = verb_left?;
    let group_by_call = find_group_by(&verb_left)?;
    let group_by_call = unwrap_or_return_none!(group_by_call);

    // Extract the grouping arguments from `group_by()`
    let group_by_args = group_by_call.arguments()?;
    let group_by_items: Vec<_> = group_by_args.items().into_iter().collect();
    if group_by_items.is_empty() {
        return Ok(None);
    }

    // `group_by()` must have no named arguments because they don't translate to
    // .by / by
    let group_by_has_named_args = group_by_args
        .items()
        .into_iter()
        .any(|x| x.is_ok_and(|a| a.name_clause().is_some()));

    if group_by_has_named_args {
        return Ok(None);
    }

    // Determine the grouping arguments text. If group_by is piped into,
    // all args are grouping vars. If it's called directly (group_by(data, grp)),
    // the first unnamed arg is the data, the rest are grouping vars.
    let group_by_is_piped = is_piped_into(&group_by_call);
    let grouping_args = get_grouping_args_text(&group_by_items, group_by_is_piped)?;
    let grouping_args = unwrap_or_return_none!(grouping_args);

    // Build the diagnostic from group_by() to ungroup()
    let range = TextRange::new(
        group_by_call.syntax().text_trimmed_range().start(),
        pipe_expr.syntax().text_trimmed_range().end(),
    );

    let body =
        format!("`group_by()` followed by `{verb_name}()` and `ungroup()` can be simplified.",);
    let suggestion = format!("Use `{verb_name}(..., {by_arg} = {grouping_args})` instead.",);

    // Build fix when group_by() has no named arguments and is piped into.
    // When group_by() is called directly (e.g. group_by(data, grp)), the data
    // argument would need to be moved into the verb, which is too complex to
    // autofix reliably.
    let has_named_group_by_args = group_by_items
        .iter()
        .any(|item| item.as_ref().is_ok_and(|a| a.name_clause().is_some()));
    let fix = if has_named_group_by_args || !group_by_is_piped {
        Fix::empty()
    } else {
        let verb_text = verb_call.to_trimmed_string();
        // Insert `by`/`.by = grouping_args` before the closing paren
        let fix_content = match verb_text.rfind(')') {
            Some(pos) => format!("{}, {by_arg} = {grouping_args})", &verb_text[..pos]),
            None => return Ok(None),
        };
        Fix {
            content: fix_content,
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(pipe_expr.syntax()),
        }
    };

    Ok(Some(Diagnostic::new(
        ViolationData::new("dplyr_group_by_ungroup".to_string(), body, Some(suggestion)),
        range,
        fix,
    )))
}

/// Check if an operator token is a pipe (`|>` or `%>%`).
fn is_pipe(operator: &RSyntaxToken) -> bool {
    let kind = operator.kind();
    kind == RSyntaxKind::PIPE || (kind == RSyntaxKind::SPECIAL && operator.text_trimmed() == "%>%")
}

/// Check if a call node receives input from a pipe (i.e., is on the right side).
fn is_piped_into(call: &RCall) -> bool {
    call.syntax()
        .prev_sibling_or_token()
        .map(|prev| {
            prev.kind() == RSyntaxKind::PIPE
                || (prev.kind() == RSyntaxKind::SPECIAL
                    && prev.as_token().is_some_and(|t| t.text_trimmed() == "%>%"))
        })
        .unwrap_or(false)
}

/// Find the `group_by()` call from the left side of the verb pipe.
/// Handles:
/// - `... |> group_by(grp)` (piped)
/// - `group_by(data, grp)` (direct call)
fn find_group_by(expr: &AnyRExpression) -> anyhow::Result<Option<RCall>> {
    // Case 1: `group_by(...)` directly
    if let Some(call) = expr.as_r_call() {
        let name = get_function_name(call.function()?);
        if name == "group_by" {
            let ns = get_function_namespace_prefix(call.function()?);
            if ns.is_none() || ns.as_deref() == Some("dplyr::") {
                return Ok(Some(call.clone()));
            }
        }
        return Ok(None);
    }

    // Case 2: `... |> group_by(...)` (pipe where right side is group_by)
    if let Some(bin) = expr.as_r_binary_expression() {
        let RBinaryExpressionFields { operator, right, .. } = bin.as_fields();
        if is_pipe(&operator?)
            && let Some(call) = right?.as_r_call()
        {
            let name = get_function_name(call.function()?);
            if name == "group_by" {
                let ns = get_function_namespace_prefix(call.function()?);
                if ns.is_none() || ns.as_deref() == Some("dplyr::") {
                    return Ok(Some(call.clone()));
                }
            }
        }
    }

    Ok(None)
}

/// Extract the text representation of grouping arguments from `group_by()`.
fn get_grouping_args_text(
    items: &[biome_rowan::SyntaxResult<RArgument>],
    is_piped: bool,
) -> anyhow::Result<Option<String>> {
    let mut grouping_parts = Vec::new();

    for (i, item) in items.iter().enumerate() {
        let arg = match item {
            Ok(a) => a,
            Err(_) => return Ok(None),
        };

        // If not piped, skip the first unnamed arg (the data argument)
        if !is_piped && i == 0 && arg.name_clause().is_none() {
            continue;
        }

        let text = match arg.value() {
            Some(v) => v.to_trimmed_string(),
            None => return Ok(None),
        };
        grouping_parts.push(text);
    }

    if grouping_parts.is_empty() {
        return Ok(None);
    }

    // Wrap in c() when there are multiple args, or when any arg uses !!!
    // (splice), since !!! expands to multiple values at runtime.
    let needs_c_wrap =
        grouping_parts.len() > 1 || grouping_parts.iter().any(|p| p.starts_with("!!!"));

    if needs_c_wrap {
        Ok(Some(format!("c({})", grouping_parts.join(", "))))
    } else {
        Ok(Some(grouping_parts.into_iter().next().unwrap()))
    }
}
