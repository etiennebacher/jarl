use crate::diagnostic::*;
use crate::utils::{
    get_arg_by_name, get_function_name, get_function_namespace_prefix, get_unnamed_args,
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
/// 1. checks whether `glue()` evaluates some R code between delimiters;
/// 2. checks whether `glue()` would error when evaluated because of incomplete
///    delimiters.
///
/// ## Why is this bad?
///
/// For 1, using `glue()` with only a constant string, e.g. `glue("abc")`, is
/// useless and less readable. You can just use the string directly.
///
/// For 2, having incomplete delimiters would error when evaluated,
/// so this indicates a bug.
///
/// Both cases do not have an automatic fix.
///
/// ## Example
///
/// ```r
/// glue("abc")
/// glue('{a}', .open = '<', .close = '>')
/// glue("{abc")
/// ```
///
/// Use instead:
/// ```r
/// "abc"
/// # For the second case, either use default delimiters {},
/// # or ensure the string contains the specified delimiters
/// # For the third case, fix the string to have complete delimiters,
/// # e.g. glue("{abc}")
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

    // TODO figure out how to handle pipes.
    if ast.has_previous_pipe() {
        return Ok(None);
    }

    let args = ast.arguments()?.items();
    let dots = get_unnamed_args(&args);

    // If there is not exactly one unnamed argument, then we can't determine
    // whether `glue()` is being used just to wrap a constant string or to
    // concatenate multiple values, so skip linting. This also avoids guessing
    // the intent of the user when they use glue() for concatenation of multiple
    // constant strings, e.g. `glue("a", "b")`.
    if dots.len() != 1 {
        return Ok(None);
    }

    let dot = &dots[0];
    let dot_value = unwrap_or_return_none!(dot.value());
    let dot_r_value = unwrap_or_return_none!(dot_value.as_any_r_value());
    let dot_text = unwrap_or_return_none!(get_string_literal_contents(
        &dot_r_value.to_trimmed_string(),
    ));

    let open_arg = get_arg_by_name(&args, ".open");
    let close_arg = get_arg_by_name(&args, ".close");
    let open_text = get_named_string_arg_text(&args, ".open")?;
    let close_text = get_named_string_arg_text(&args, ".close")?;

    if (open_arg.is_some() && open_text.is_none()) || (close_arg.is_some() && close_text.is_none())
    {
        return Ok(None);
    }

    let open = open_text.as_deref().unwrap_or("{");
    let close = close_text.as_deref().unwrap_or("}");

    let diagnostic = if has_incomplete_delimiters(&dot_text, open, close) {
        Some(Diagnostic::new(
            ViolationData::new(
                "glue".to_string(),
                "This `glue()` call contains incomplete delimiters and would error when evaluated."
                    .to_string(),
                None,
            ),
            ast.syntax().text_trimmed_range(),
            Fix::empty(),
        ))
    } else if !dot_text.contains(open) && !dot_text.contains(close) {
        Some(Diagnostic::new(
            ViolationData::new(
                "glue".to_string(),
                "This `glue()` call isn't necessary because it performs no interpolation."
                    .to_string(),
                None,
            ),
            ast.syntax().text_trimmed_range(),
            Fix::empty(),
        ))
    } else {
        None
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

fn has_incomplete_delimiters(text: &str, open: &str, close: &str) -> bool {
    if open.is_empty() || close.is_empty() {
        return false;
    }

    // In glue, doubled delimiters are escape sequences for literal characters
    // and they must be skipped before checking for single delimiters.
    let escaped_open = format!("{open}{open}");
    let escaped_close = format!("{close}{close}");

    let mut balance = 0;
    let mut index = 0;

    while index < text.len() {
        let slice = &text[index..];

        if slice.starts_with(&escaped_open) {
            index += escaped_open.len();
            continue;
        }

        if slice.starts_with(&escaped_close) {
            index += escaped_close.len();
            continue;
        }

        if slice.starts_with(open) {
            balance += 1;
            index += open.len();
            continue;
        }

        if slice.starts_with(close) {
            if balance == 0 {
                return true;
            }
            balance -= 1;
            index += close.len();
            continue;
        }

        index += 1;
    }

    balance != 0
}
