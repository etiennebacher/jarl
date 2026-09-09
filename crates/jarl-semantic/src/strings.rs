//! String-literal parsing shared across jarl crates.
//!
//! Unquotes R string literals — standard (`"x"` / `'x'`) and raw
//! (`r"(x)"`, `R'-[x]-'`, …) — into their contents. oak's
//! `RStringValueExt::string_text` only strips surrounding quotes and does not
//! understand raw strings, so these helpers fill that gap. Reused by the
//! `glue` lint in jarl-core and by the custom-delimiter interpolation pass.

/// Parse string literal content from its raw token text (including quotes).
/// Handles both standard strings (`"abc"` or `'abc'`) and raw strings
/// (`r"(abc)"` or `R'-[abc]-'`). Returns the unquoted content as a `String`
/// if parsing succeeds, `None` otherwise.
pub fn get_string_literal_contents(text: &str) -> Option<String> {
    parse_standard_string(text)
        .or_else(|| parse_raw_string(text))
        .map(|content| content.to_string())
}

/// Parse a standard string literal: `"content"` or `'content'`.
/// Returns the unquoted content if the string has matching quotes, `None` otherwise.
fn parse_standard_string(text: &str) -> Option<&str> {
    let quote = text.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let content = text.strip_prefix(quote)?;
    content.strip_suffix(quote)
}

/// Parse a raw string literal: `r"(content)"`, `r'-[content]-'`, etc. (R v4.0+).
/// Handles dashes before the delimiter to avoid early termination.
/// Returns the content between delimiters if parsing succeeds, `None` otherwise.
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
