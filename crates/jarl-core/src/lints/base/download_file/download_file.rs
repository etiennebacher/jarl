use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{Formals, get_arg};
use air_r_syntax::*;
use biome_rowan::AstNode;

// The order of `download.file()`'s arguments
const FORMALS_DOWNLOAD_FILE: Formals = &[
    "url", "destfile", "method", "quiet", "mode", "cacheOK", "extra", "headers",
];

// These are the suffixes that indicate a binary file on Windows.
const WINDOWS_BINARY_SUFFIXES: &[&str] = &[
    ".gz", ".bz2", ".xz", ".tgz", ".zip", ".jar", ".rda", ".rds", ".RData",
];

/// <!-- docs: start -->
/// Version added: 0.0.24
///
/// ## What it does
///
/// Checks for missing or text-mode `mode` arguments in `download.file()`, and
/// for `mode` arguments ignored by `method = "curl"` or `method = "wget"`.
///
/// ## Why is this bad?
///
/// Text modes `"w"` and `"a"` can corrupt binary downloads on Windows. Use
/// `"wb"` or `"ab"` instead. With the external `curl` and `wget` methods,
/// `mode` is ignored and should be omitted.
///
/// When `mode` is missing, literal URLs ending in `.gz`, `.bz2`, `.xz`, `.tgz`,
/// `.zip`, `.jar`, `.rda`, `.rds`, or `.RData` are exempt because R automatically
/// uses binary mode for them on Windows. The check is case-sensitive and uses
/// the URL after percent-decoding, not the destination filename. Explicit text
/// modes are still reported for these URLs.
///
/// ## Example
///
/// ```r
/// download.file(url, destfile)
/// download.file(url, destfile, mode = "a")
/// download.file(url, destfile, method = "curl", mode = "wb")
/// ```
///
/// Use instead:
/// ```r
/// download.file(url, destfile, mode = "wb")
/// download.file(url, destfile, mode = "ab")
/// download.file(url, destfile, method = "curl")
/// ```
///
/// ## References
///
/// See `?download.file`
/// <!-- docs: end -->
pub fn download_file(
    ast: &RCall,
    fn_name: &str,
    ns_prefix: Option<&str>,
) -> anyhow::Result<Option<Diagnostic>> {
    if fn_name != "download.file" {
        return Ok(None);
    }
    match ast.function()? {
        AnyRExpression::RIdentifier(_) => {}
        AnyRExpression::RNamespaceExpression(_) if ns_prefix == Some("utils::") => {}
        _ => return Ok(None),
    }

    let method = get_arg(ast, FORMALS_DOWNLOAD_FILE, "method")
        .and_then(|arg| arg.value())
        .and_then(literal_string);
    let mode = get_arg(ast, FORMALS_DOWNLOAD_FILE, "mode").and_then(|arg| arg.value());

    let (msg, suggestion) = if matches!(method.as_deref(), Some("curl" | "wget")) {
        // External download commands never use mode, even when its value is dynamic.
        if mode.is_none() {
            return Ok(None);
        }
        (
            "`download.file()` ignores `mode` with `method = 'curl'` or `method = 'wget'`.",
            "Remove the mode argument.",
        )
    } else if let Some(mode) = mode {
        // If `mode` is provided, check if its value is "w" or "a".
        match literal_string(mode).as_deref() {
            Some("w") => (
                "`download.file()` with `mode = 'w'` can cause portability issues on Windows.",
                "Use mode = 'wb' instead.",
            ),
            Some("a") => (
                "`download.file()` with `mode = 'a'` can cause portability issues on Windows.",
                "Use mode = 'ab' instead.",
            ),
            _ => return Ok(None),
        }
    } else {
        // R checks the decoded URL suffix only when mode is missing.
        if let Some(url) = get_arg(ast, FORMALS_DOWNLOAD_FILE, "url")
            .and_then(|arg| arg.value())
            .and_then(literal_string)
        {
            let url = percent_encoding::percent_decode_str(&url).decode_utf8_lossy();
            if WINDOWS_BINARY_SUFFIXES
                .iter()
                .any(|suffix| url.ends_with(suffix))
            {
                return Ok(None);
            }
        }
        (
            "`download.file()` without explicit `mode` can cause portability issues on Windows.",
            "Use mode = 'wb' instead.",
        )
    };

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::DownloadFile,
            msg.to_string(),
            Some(suggestion.to_string()),
        ),
        ast.syntax().text_trimmed_range(),
        Fix::empty(),
    )))
}

// Extract a statically known string value for the method, mode, and URL checks.
// Return None when the expression is not a string literal or cannot be decoded.
fn literal_string(mut expr: AnyRExpression) -> Option<String> {
    while let AnyRExpression::RParenthesizedExpression(paren) = expr {
        expr = paren.body().ok()?;
    }

    // Require a string literal and remove its ordinary or raw-string delimiters.
    let string = expr.as_any_r_value()?.as_r_string_value()?;
    let content = string.inner_string_text().ok()?;

    // Raw strings preserve backslashes. Ordinary strings without escapes also
    // already contain the value to compare.
    if string.value().ok()?.kind() == RSyntaxKind::R_RAW_STRING_LITERAL
        || !content.text().contains('\\')
    {
        return Some(content.to_string());
    }

    // Decode escapes in ordinary strings so spellings such as "\x77" match "w".
    let mut decoded = String::with_capacity(content.text().len());
    let mut chars = content.text().chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            decoded.push(c);
            continue;
        }
        let escaped = match chars.next()? {
            'a' => '\u{7}',
            'b' => '\u{8}',
            'f' => '\u{c}',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'v' => '\u{b}',
            c @ ('\\' | '\'' | '"' | '\n') => c,
            c @ ('0'..='7' | 'x' | 'u' | 'U') => {
                // Each numeric escape has its own base and maximum digit count.
                let (radix, limit, mut digits) = match c {
                    'x' => (16, 2, String::new()),
                    'u' => (16, 4, String::new()),
                    'U' => (16, 8, String::new()),
                    _ => (8, 3, c.to_string()),
                };
                // Unicode escapes also allow braces, as in \u{0077}.
                let braced = matches!(c, 'u' | 'U') && chars.next_if_eq(&'{').is_some();
                while digits.len() < limit {
                    let Some(digit) = chars.next_if(|c| c.is_digit(radix)) else {
                        break;
                    };
                    digits.push(digit);
                }
                if braced && chars.next()? != '}' {
                    return None;
                }
                char::from_u32(u32::from_str_radix(&digits, radix).ok()?)?
            }
            _ => return None,
        };
        // R strings cannot contain a NUL byte.
        if escaped == '\0' {
            return None;
        }
        decoded.push(escaped);
    }
    Some(decoded)
}
