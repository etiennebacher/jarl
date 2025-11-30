use crate::diagnostic::*;
use crate::utils::{get_arg_by_name_then_position, get_function_name};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for usage of `download.file()` with `mode = "a"` or `mode = "w"`.
///
/// ## Why is this bad?
///
/// `mode = "a"` or `mode = "w"` can generate broken files on Windows.
/// `download.file()` documentation recommends using `mode = "wb"` and
/// `mode = "a"` instead. If `method = "curl"` or `method = "wget"`, no mode
/// should be provided as it will be ignored.
///
/// ## Example
///
/// ```r
/// download.file(x = my_url)
/// download.file(x = my_url, mode = "w")
/// ```
///
/// Use instead:
/// ```r
/// download.file(x = my_url, mode = "wb")
/// ```
///
/// ## References
///
/// See `?download.file`
pub fn download_file(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let fn_name = get_function_name(function);

    if fn_name != "download.file" {
        return Ok(None);
    }

    let args = ast.arguments()?.items();
    let method = get_arg_by_name_then_position(&args, "method", 3);
    let mode_arg = get_arg_by_name_then_position(&args, "mode", 5);

    // Check if method is wget or curl - if so, mode is ignored anyway
    if let Some(method) = method.and_then(|arg| arg.value())
        && let Some(method_value) = method.as_any_r_value()
        && let Some(method_str) = method_value.as_r_string_value()
    {
        let method_val = method_str.to_trimmed_string();
        if method_val == "\"wget\""
            || method_val == "'wget'"
            || method_val == "\"curl\""
            || method_val == "'curl'"
        {
            return Ok(None);
        }
    }

    // Extract mode value if present
    let mode_value = match mode_arg.and_then(|arg| arg.value()) {
        Some(mode_val) => {
            if let Some(r_value) = mode_val.as_any_r_value() {
                if let Some(str_value) = r_value.as_r_string_value() {
                    Some(str_value.to_trimmed_string())
                } else {
                    return Ok(None);
                }
            } else {
                return Ok(None);
            }
        }
        None => None,
    };

    let (msg, suggestion) = match mode_value.as_deref() {
        Some("\"w\"") | Some("'w'") => (
            "`download.file()` with `mode = 'w'` can cause portability issues on Windows.",
            "Use mode = 'wb' instead.",
        ),
        Some("\"a\"") | Some("'a'") => (
            "`download.file()` with `mode = 'a'` can cause portability issues on Windows.",
            "Use mode = 'ab' instead.",
        ),
        // We returned early if method = "curl" or method = "wget", so we know
        // that this default mode value isn't ignored.
        None => (
            "`download.file()` without explicit `mode` can cause portability issues on Windows.",
            "Use mode = 'wb' instead.",
        ),
        _ => return Ok(None),
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "download_file".to_string(),
            msg.to_string(),
            Some(suggestion.to_string()),
        ),
        range,
        Fix::empty(),
    );

    Ok(Some(diagnostic))
}
