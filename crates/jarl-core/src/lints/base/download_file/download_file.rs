use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{Formals, get_arg};
use air_r_syntax::*;
use biome_rowan::AstNode;

const FORMALS_DOWNLOAD_FILE: Formals = &[
    "url", "destfile", "method", "quiet", "mode", "cacheOK", "extra", "headers",
];

const WINDOWS_BINARY_SUFFIXES: &[&str] = &[
    ".gz", ".bz2", ".xz", ".tgz", ".zip", ".jar", ".rda", ".rds", ".RData",
];

/// <!-- docs: start -->
/// Version added: 0.0.24
///
/// ## What it does
///
/// Checks for missing or non-binary `mode` arguments in `download.file()`, and
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

    let binary_mode_message = (
        "`download.file()` without an explicit binary `mode` can cause portability issues on Windows.",
        "Use mode = 'wb' or mode = 'ab' instead.",
    );
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
        match literal_string(mode).as_deref() {
            Some("wb" | "ab") | None => return Ok(None),
            Some(_) => binary_mode_message,
        }
    } else {
        // R checks the decoded URL suffix only when mode is missing (on Windows only).
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
        binary_mode_message
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

// Extract the literal text used to check the method, mode, and URL arguments.
fn literal_string(mut expr: AnyRExpression) -> Option<String> {
    // Parentheses do not change the literal's value.
    while let AnyRExpression::RParenthesizedExpression(paren) = expr {
        expr = paren.body().ok()?;
    }

    // Remove ordinary or raw-string delimiters.
    let string = expr.as_any_r_value()?.as_r_string_value()?;
    Some(string.inner_string_text().ok()?.to_string())
}
