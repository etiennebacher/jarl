use crate::diagnostic::*;
use crate::utils::{get_arg_by_name, get_function_name, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for calls to `stop()` that display the call in the error message,
/// either because `call.` is not set (it defaults to `TRUE`) or because it is
/// explicitly set to `TRUE`.
///
/// ## Why is this bad?
///
/// By default, `stop()` shows the call that triggered the error in the
/// message. This can be noisy and lead to confusion if the user didn't directly
/// call the function that threw the error.
///
/// This rule has an unsafe automatic fix (unsafe because it may break tests
/// that rely on the exact error message).
///
/// ## Example
///
/// ```r
/// internal_function <- function(x) {
///   if (x < 5) {
///     stop("x lower than 5")
///   }
/// }
///
/// external_function<- function(x) {
///   out <- internal_function(x)
///   # do something with `out`...
/// }
///
/// external_function(1)
/// #> Error in `internal_function()`:
/// #> x lower than 5
/// ```
///
/// In this case, the error message is slightly confusing because the user never
/// called `internal_function()` directly. With `call. = FALSE` instead:
///
/// ```r
/// internal_function <- function(x) {
///   if (x < 5) {
///     stop("x lower than 5", call. = FALSE)
///   }
/// }
///
/// external_function<- function(x) {
///   out <- internal_function(x)
///   # do something with `out`...
/// }
///
/// external_function(1)
/// #> Error:
/// #> x lower than 5
/// ```
///
/// ## References
///
/// * https://design.tidyverse.org/err-call.html
pub fn condition_call(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let fn_name = get_function_name(function);

    if fn_name != "stop" {
        return Ok(None);
    }

    let arguments = ast.arguments()?;
    let args = arguments.items();
    let call_arg = get_arg_by_name(&args, "call.");

    let to_skip = node_contains_comments(ast.syntax());

    let (body, suggestion, fix) = match call_arg {
        // Either we have `call. =`, which defaults to TRUE, or we have `call.`
        // with an explicit value, in which case we report only when this value
        // is a literal TRUE
        Some(arg) => {
            let value = arg.value();
            if let Some(ref value) = value
                && value.as_r_true_expression().is_none()
            {
                // `call. = FALSE` (the desired state) or a non-literal value
                // we can't reason about.
                return Ok(None);
            };

            let (value_range_start, value_range_end) = if let Some(ref value) = value {
                let range = value.syntax().text_trimmed_range();
                (range.start(), range.end())
            } else {
                let range = arg
                    .name_clause()
                    .unwrap()
                    .eq_token()
                    .unwrap()
                    .text_trimmed_range();
                (
                    range.start().checked_add(1.into()).unwrap(),
                    range.end().checked_add(1.into()).unwrap(),
                )
            };
            (
                "Including the call in the error message may lead to confusion.".to_string(),
                "Use `call. = FALSE` instead.".to_string(),
                Fix {
                    content: "FALSE".to_string(),
                    start: value_range_start.into(),
                    end: value_range_end.into(),
                    to_skip,
                },
            )
        }
        // `call.` is absent: it defaults to `TRUE`, so insert `call. = FALSE`.
        None => {
            let last_arg = args.into_iter().filter_map(|x| x.ok()).last();
            let (start, content) = match last_arg {
                Some(arg) => (
                    usize::from(arg.syntax().text_trimmed_range().end()),
                    ", call. = FALSE".to_string(),
                ),
                None => (
                    usize::from(arguments.l_paren_token()?.text_trimmed_range().end()),
                    "call. = FALSE".to_string(),
                ),
            };
            (
                "`stop()` includes the call in the error message by default, which may lead to confusion.".to_string(),
                "Add `call. = FALSE` to hide it.".to_string(),
                Fix { content, start, end: start, to_skip },
            )
        }
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new("condition_call".to_string(), body, Some(suggestion)),
        range,
        fix,
    );

    Ok(Some(diagnostic))
}
