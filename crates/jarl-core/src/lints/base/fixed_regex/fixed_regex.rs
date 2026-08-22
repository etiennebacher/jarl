use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{Formals, get_arg, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

const FORMALS_GSUB: Formals = &[
    "pattern",
    "replacement",
    "x",
    "ignore.case",
    "perl",
    "fixed",
    "useBytes",
];
const FORMALS_GREP: Formals = &[
    "pattern",
    "x",
    "ignore.case",
    "perl",
    "value",
    "fixed",
    "useBytes",
    "invert",
];
const FORMALS_GREPL: Formals = &["pattern", "x", "ignore.case", "perl", "fixed", "useBytes"];
const FORMALS_REGEXPR: Formals = &[
    "pattern",
    "text",
    "ignore.case",
    "perl",
    "fixed",
    "useBytes",
];

pub struct FixedRegex;

/// Version added: 0.3.0
///
/// ## What it does
///
/// Checks for regex functions (`grep`, `grepl`, `gsub`, `sub`, `regexpr`,
/// `gregexpr`, `regexec`) called with a pattern that contains no special
/// regex characters and without `fixed = TRUE`.
///
/// ## Why is this bad?
///
/// When a pattern contains no special regex characters, using `fixed = TRUE`
/// provides a significant performance boost because it uses simple string
/// matching instead of regex engine pattern matching.
///
/// This rule has a safe automatic fix.
///
/// ## Example
///
/// ```r
/// grep("hello", x)
/// gsub("world", "universe", text)
/// ```
///
/// Use instead:
/// ```r
/// grep("hello", x, fixed = TRUE)
/// gsub("world", "universe", text, fixed = TRUE)
/// ```
///
/// ## References
///
/// See `?grep` and `?fixed`
impl Violation for FixedRegex {
    fn rule(&self) -> Rule {
        Rule::FixedRegex
    }
    fn body(&self) -> String {
        "Pattern contains no regex special characters but `fixed = TRUE` is not set.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Add `fixed = TRUE` for better performance.".to_string())
    }
}

pub fn fixed_regex(ast: &RCall, fn_name: &str) -> anyhow::Result<Option<Diagnostic>> {
    let args = ast.arguments()?.items();

    let formals = match fn_name {
        "gsub" | "sub" => FORMALS_GSUB,
        "grep" => FORMALS_GREP,
        "grepl" => FORMALS_GREPL,
        "regexpr" | "gregexpr" | "regexec" => FORMALS_REGEXPR,
        _ => return Ok(None),
    };

    // Check if `fixed` is already explicitly supplied (by name or position).
    // If the user wrote `fixed = TRUE`, `fixed = FALSE`, or `fixed = some_var`,
    // they are making a deliberate choice and we should not second-guess it.
    if get_arg(ast, formals, "fixed").is_some() {
        return Ok(None);
    }

    // Check if ignore.case is explicitly supplied (implies regex interpretation)
    if get_arg(ast, formals, "ignore.case").is_some() {
        return Ok(None);
    }

    let pattern_arg = unwrap_or_return_none!(get_arg(ast, formals, "pattern"));
    let pattern_value = unwrap_or_return_none!(pattern_arg.value());

    // Check if the pattern is a string literal
    let r_value = unwrap_or_return_none!(pattern_value.as_any_r_value());
    let string_value = unwrap_or_return_none!(r_value.as_r_string_value());
    let pattern_string = string_value.to_trimmed_string();

    // Remove outer quotes to get the actual pattern
    let pattern_content = pattern_string.trim_matches(|c| c == '"' || c == '\'');

    // Check if the pattern is fixed (no special regex characters)
    if !is_fixed_pattern(pattern_content) {
        return Ok(None);
    }

    // Pattern is fixed but `fixed` is not set — build fix by adding `fixed = TRUE`.
    let args_text = args
        .into_iter()
        .filter_map(|arg| arg.ok())
        .map(|arg| arg.syntax().text_trimmed().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let fixed_content = format!("{}({}, fixed = TRUE)", fn_name, args_text);

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        FixedRegex,
        range,
        Fix::new(range, fixed_content, node_contains_comments(ast.syntax())),
    );

    Ok(Some(diagnostic))
}

/// Check if a pattern string contains no unescaped regex special characters
fn is_fixed_pattern(pattern: &str) -> bool {
    const REGEX_CHARS: &[u8; 12] = b".*+?[{()|^$\\";

    pattern.bytes().all(|b| !REGEX_CHARS.contains(&b))
}
