use crate::diagnostic::*;
use crate::utils::{
    get_arg_by_name, get_function_name, get_function_namespace_prefix, get_named_args,
    get_unnamed_args,
};
use crate::utils_ast::AstNodeExt;
use air_r_syntax::*;
use biome_rowan::AstNode;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Multiple checks for `glue()`:
///
/// 1. checks whether `glue()` has exactly one unnamed argument and no other named
///    arguments (in which case `glue()` is not needed);
/// 2. checks whether `glue()` has exactly one unnamed argument with only `.open`
///    and `.close` named arguments, and the string does not contain both specified
///    delimiters (in which case using `.open` and `.close` is not needed).
///
/// ## Why is this bad?
///
/// For 1, using `glue()` with only a constant string, e.g. `glue("abc")`, is
/// useless and less readable. You can just use the string directly.
///
/// For 2, specifying `.open` and `.close` delimiters when the string does not
/// contain those delimiters means `glue()` will not perform any interpolation,
/// making the function call unnecessary.
///
/// Both cases do not have an automatic fix.
///
/// ## Example
///
/// ```r
/// glue("abc")
/// glue('{a}', .open = '<', .close = '>')
/// ```
///
/// Use instead:
/// ```r
/// "abc"
/// # For the second case, either use default delimiters {}, or ensure the string contains the specified delimiters
/// ```
///
/// ## References
///
/// See `?glue::glue`
pub fn glue(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let fn_name = get_function_name(ast.function()?);
    let fn_ns = get_function_namespace_prefix(ast.function()?);

    // Only trigger on `glue()` or `glue::glue()`
    if fn_name != "glue" {
        return Ok(None);
    }
    if let Some(ref ns) = fn_ns
        && ns != "glue::"
    {
        return Ok(None);
    }

    // Don't know how to handle pipes for now.
    if ast.has_previous_pipe() {
        return Ok(None);
    }

    let args = ast.arguments()?.items();
    let named = get_named_args(&args);
    let dots = get_unnamed_args(&args);

    // If there is not exactly one unnamed argument, then we can't determine whether
    // `glue()` is being used just to wrap a constant string or to concatenate
    // multiple values, so skip linting.
    if dots.len() != 1 {
        return Ok(None);
    }

    let dot = &dots[0];
    let dot_value = unwrap_or_return_none!(dot.value());
    let dot_r_value = unwrap_or_return_none!(dot_value.as_any_r_value());
    let dot_text = unwrap_or_return_none!(get_string_literal_contents(
        &dot_r_value.to_trimmed_string(),
    ));

    let diagnostic = if named.is_empty() {
        Some(Diagnostic::new(
            ViolationData::new(
                "glue".to_string(),
                "glue() with a constant string performs no interpolation.".to_string(),
                None,
            ),
            ast.syntax().text_trimmed_range(),
            Fix::empty(),
        ))
    } else {
        let is_only_open_close = named.iter().all(|arg| {
            arg.name_clause()
                .and_then(|nc| nc.name().ok())
                .map(|name| matches!(name.to_trimmed_string().as_str(), ".open" | ".close"))
                .unwrap_or(false)
        });

        if !is_only_open_close {
            None
        } else {
            let open_text = get_named_string_arg_text(&args, ".open")?;
            let close_text = get_named_string_arg_text(&args, ".close")?;

            // If only one of `.open` or `.close` is provided, `glue()` may still be
            // valid because the other delimiter uses its default value.
            if let (Some(open), Some(close)) = (open_text, close_text) {
                if !open.is_empty()
                    && !close.is_empty()
                    && (!dot_text.contains(&open) || !dot_text.contains(&close))
                {
                    Some(Diagnostic::new(
                        ViolationData::new(
                            "glue".to_string(),
                            "Using glue() with .open and .close when the string does not contain the specified delimiters performs no interpolation.".to_string(),
                            None,
                        ),
                        ast.syntax().text_trimmed_range(),
                        Fix::empty(),
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        }
    };

    Ok(diagnostic)
}

/// Extract and parse the string content from a named argument.
/// Returns `Ok(None)` if the argument is not found, or if it is not a string literal.
/// Returns `Ok(Some(content))` with the unquoted string content on success.
fn get_named_string_arg_text(args: &RArgumentList, name: &str) -> anyhow::Result<Option<String>> {
    let arg = match get_arg_by_name(args, name) {
        Some(arg) => arg,
        None => return Ok(None),
    };

    let value = unwrap_or_return_none!(arg.value());
    let r_value = unwrap_or_return_none!(value.as_any_r_value());
    let string_value = unwrap_or_return_none!(r_value.as_r_string_value());

    Ok(get_string_literal_contents(
        &string_value.to_trimmed_string(),
    ))
}

/// Parse string literal content from its raw token text (including quotes).
/// Handles both standard strings ("abc" or 'abc') and raw strings (r"(abc)" or R'-[abc]-').
/// Returns the unquoted content as a String if parsing succeeds, None otherwise.
fn get_string_literal_contents(text: &str) -> Option<String> {
    parse_standard_string(text)
        .or_else(|| parse_raw_string(text))
        .map(|content| content.to_string())
}

/// Parse a standard string literal: "content" or 'content'.
/// Returns the unquoted content if the string has matching quotes, None otherwise.
fn parse_standard_string(text: &str) -> Option<&str> {
    let quote = text.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let content = text.strip_prefix(quote)?;
    content.strip_suffix(quote)
}

/// Parse a raw string literal: r"(content)", r'-[content]-', etc. (R v4.0+)
/// Handles dashes before the delimiter to avoid early termination.
/// Returns the content between delimiters if parsing succeeds, None otherwise.
fn parse_raw_string(text: &str) -> Option<&str> {
    let raw_prefix = text.chars().next()?;
    if raw_prefix != 'r' && raw_prefix != 'R' {
        return None;
    }

    let rest = text.strip_prefix(raw_prefix)?;
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let rest = rest.strip_prefix(quote)?;
    let after_dashes = rest.trim_start_matches('-');
    let leading_dashes = &rest[..rest.len() - after_dashes.len()];

    let open_brace = after_dashes.chars().next()?;
    let close_brace = match open_brace {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        _ => return None,
    };

    let body_and_suffix = after_dashes.strip_prefix(open_brace)?;
    let expected_closing_fence = format!("{}{}{}", close_brace, leading_dashes, quote);
    body_and_suffix.strip_suffix(&expected_closing_fence)
}
