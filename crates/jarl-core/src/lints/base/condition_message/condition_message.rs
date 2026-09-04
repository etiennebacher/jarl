use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{
    get_arg_by_name, get_function_name, get_nested_functions_content, node_contains_comments,
};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for calls to `stop()` or `warning()` that contain `paste0()`.
///
/// ## Why is this bad?
///
/// By default, `stop()` and `warning()` concatenate elements in the message
/// without any separator. Using `paste0()` is therefore not needed.
///
/// ## Example
///
/// ```r
/// stop(paste0('hello ', 'there'))
/// warning(paste0('hello ', 'there'))
/// ```
///
/// ```r
/// stop('hello ', 'there')
/// warning('hello ', 'there')
/// ```
pub fn condition_message(ast: &RCall, fn_name: &str) -> anyhow::Result<Option<Diagnostic>> {
    if fn_name != "stop" && fn_name != "warning" {
        return Ok(None);
    }

    // When `paste0()` is a direct argument of the call, the other arguments must
    // be kept in the fix, so remember which argument holds it.
    let (inner_content, outer_syntax, paste_arg_index) =
        if let Some((inner_content, index)) = get_direct_nested_paste0_content(ast)? {
            (inner_content, ast.syntax().clone(), Some(index))
        } else {
            let (inner_content, outer_syntax) = unwrap_or_return_none!(
                get_nested_functions_content(ast, fn_name, fn_name, "paste0")?
            );
            (inner_content, outer_syntax, None)
        };

    // `stop()` doesn't have equivalents for recycle0 or collapse args, so bail
    // early
    if paste0_has_unsupported_args(&outer_syntax)? {
        return Ok(None);
    }

    let extra_args = ast
        .arguments()?
        .items()
        .into_iter()
        .enumerate()
        .filter(|(index, _)| Some(*index) != paste_arg_index)
        .filter_map(|(_, arg)| Some(arg.ok()?.to_trimmed_string()));
    let new_content = std::iter::once(inner_content)
        .chain(extra_args)
        .collect::<Vec<_>>()
        .join(", ");

    let range = outer_syntax.text_trimmed_range();
    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::ConditionMessage,
            format!("`{}(paste0(...))` can be simplified.", fn_name),
            Some(format!("Use `{}(...)` instead.", fn_name)),
        ),
        range,
        Fix::new(
            range,
            format!("{}({})", fn_name, new_content),
            node_contains_comments(&outer_syntax),
        ),
    )))
}

/// Returns the content of a `paste0()` call used as a direct argument, along
/// with the index of the argument holding it.
fn get_direct_nested_paste0_content(call: &RCall) -> anyhow::Result<Option<(String, usize)>> {
    let argument = call
        .arguments()?
        .items()
        .into_iter()
        .enumerate()
        .find(|(_, arg)| arg.as_ref().is_ok_and(|arg| arg.name_clause().is_none()));
    let (index, argument) = unwrap_or_return_none!(argument);
    let inner = unwrap_or_return_none!(argument?.value());
    let inner_call = unwrap_or_return_none!(inner.as_r_call());

    if get_function_name(inner_call.function()?) != "paste0" {
        return Ok(None);
    }

    let inner_content = inner_call.arguments()?.items().into_syntax().to_string();
    Ok(Some((inner_content, index)))
}

/// Whether the `paste0()` call in `node` uses arguments that `stop()` and
/// `warning()` have no equivalent for.
fn paste0_has_unsupported_args(node: &RSyntaxNode) -> anyhow::Result<bool> {
    let paste_call = node
        .descendants()
        .filter_map(RCall::cast)
        .find(|call| call.function().ok().map(get_function_name).as_deref() == Some("paste0"));
    let Some(paste_call) = paste_call else {
        return Ok(false);
    };
    let paste_args = paste_call.arguments()?.items();
    Ok(get_arg_by_name(&paste_args, "collapse").is_some()
        || get_arg_by_name(&paste_args, "recycle0").is_some())
}
