use crate::diagnostic::*;
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
pub fn condition_message(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let orig_fn = get_function_name(ast.function()?);
    if orig_fn != "stop" && orig_fn != "warning" {
        return Ok(None);
    }

    let (inner_content, outer_syntax) =
        unwrap_or_return_none!(get_nested_functions_content(ast, &orig_fn, "paste0")?);

    // `stop()` doesn't have equivalents for recycle0 or collapse args, so bail
    // early
    if let Some(paste_call) = outer_syntax
        .descendants()
        .filter_map(RCall::cast)
        .find(|call| call.function().ok().map(get_function_name).as_deref() == Some("paste0"))
    {
        let paste_args = paste_call.arguments()?.items();
        if get_arg_by_name(&paste_args, "collapse").is_some()
            || get_arg_by_name(&paste_args, "recycle0").is_some()
        {
            return Ok(None);
        }
    }

    let args = ast.arguments()?.items();
    let call_arg = get_arg_by_name(&args, "call.");
    let domain_arg = get_arg_by_name(&args, "domain");

    // In warning() only
    let immediate_arg = get_arg_by_name(&args, "immediate.");
    let nobreaks_arg = get_arg_by_name(&args, "noBreaks.");

    let extra_args = [call_arg, domain_arg, immediate_arg, nobreaks_arg]
        .into_iter()
        .flatten()
        .map(|arg| arg.to_trimmed_string());
    let new_content = std::iter::once(inner_content)
        .chain(extra_args)
        .collect::<Vec<_>>()
        .join(", ");

    let range = outer_syntax.text_trimmed_range();
    Ok(Some(Diagnostic::new(
        ViolationData::new(
            "condition_message".to_string(),
            format!("`{}(paste0(...))` can be simplified.", orig_fn),
            Some(format!("Use `{}(...)` instead.", orig_fn)),
        ),
        range,
        Fix {
            content: format!("{}({})", orig_fn, new_content),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(&outer_syntax),
        },
    )))
}
