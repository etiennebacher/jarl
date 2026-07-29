use crate::diagnostic::*;
use crate::utils::get_function_name;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct NonportablePath;

const SKIPPED_ARGUMENT_NAMES: &[&str] = &["format", "pattern"];

// These calls commonly use strings as regular-expression or date-format
// specifications rather than file paths.
const SKIPPED_FUNCTIONS: &[&str] = &[
    "agrep",
    "agrepl",
    "as.Date",
    "as.POSIXct",
    "as.POSIXlt",
    "format",
    "grep",
    "grepRaw",
    "grepl",
    "grepv",
    "gregexec",
    "gregexpr",
    "gsub",
    "regexec",
    "regexpr",
    "str_count",
    "str_detect",
    "str_ends",
    "str_extract",
    "str_extract_all",
    "str_locate",
    "str_locate_all",
    "str_match",
    "str_match_all",
    "str_remove",
    "str_remove_all",
    "str_replace",
    "str_replace_all",
    "str_split",
    "str_split_fixed",
    "str_starts",
    "str_subset",
    "str_view",
    "str_which",
    "strftime",
    "strptime",
    "strsplit",
    "sub",
];

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for likely file paths constructed with hard-coded `/` or `\\`
/// separators.
///
/// ## Why is this bad?
///
/// Hard-coded path separators may not work consistently across operating
/// systems. Use `file.path()` to construct portable paths.
///
/// This rule is disabled by default because this heuristic can also match
/// regular expressions and other strings that are not file paths.
///
/// This rule uses a conservative heuristic: a separator must be followed by a
/// path component containing at least two characters. URLs, root paths, and
/// strings containing characters that are generally invalid in paths are
/// ignored. Strings inside known regular-expression and date-formatting
/// functions, or arguments named `pattern` or `format`, are also ignored.
///
/// ## Example
///
/// ```r
/// path <- "data/raw/input.csv"
/// ```
///
/// Use instead:
/// ```r
/// path <- file.path("data", "raw", "input.csv")
/// ```
///
/// ## References
///
/// See `?file.path`.
impl Violation for NonportablePath {
    fn name(&self) -> String {
        "nonportable_path".to_string()
    }

    fn body(&self) -> String {
        "Hard-coded path separators are not portable.".to_string()
    }

    fn suggestion(&self) -> Option<String> {
        Some("Use `file.path()` to construct the path.".to_string())
    }
}

pub fn nonportable_path(ast: &AnyRValue) -> anyhow::Result<Option<Diagnostic>> {
    let string = unwrap_or_return_none!(ast.as_r_string_value());

    // The path heuristic cannot distinguish these strings reliably from paths,
    // so skip contexts whose arguments have a different, well-known meaning.
    if is_skipped_context(string) {
        return Ok(None);
    }

    let token = unwrap_or_return_none!(string.content_token());
    let open = string.open_token()?;
    let content = token.text_trimmed();

    // Syntax tokens retain R's escape sequences. Decode them before checking
    // separators so `\\` is treated as a path separator but `\n` is not.
    let path = if open.text_trimmed().starts_with(['r', 'R']) {
        content.to_string()
    } else {
        unwrap_or_return_none!(decode_standard_string(content))
    };

    if !is_nonportable_path(&path) {
        return Ok(None);
    }

    Ok(Some(Diagnostic::new(
        NonportablePath,
        token.text_range(),
        Fix::empty(),
    )))
}

fn decode_standard_string(content: &str) -> Option<String> {
    let mut decoded = String::with_capacity(content.len());
    let mut chars = content.chars();

    while let Some(character) = chars.next() {
        if character != '\\' {
            decoded.push(character);
            continue;
        }

        let escaped = chars.next()?;
        decoded.push(match escaped {
            // A doubled backslash represents the actual separator in a
            // Windows path; the other branches below represent non-separator
            // characters or control characters.
            '\\' => '\\',
            '\'' => '\'',
            '"' => '"',
            'a' => '\x07',
            'b' => '\x08',
            'f' => '\x0c',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'v' => '\x0b',
            // Skip uncommon numeric and Unicode escapes rather than guessing
            // their decoded value and risking a false positive.
            _ => return None,
        });
    }

    Some(decoded)
}

fn is_nonportable_path(path: &str) -> bool {
    // URLs and nested quoted strings are not useful candidates for this rule.
    if path.starts_with(['\'', '"']) || has_url_scheme(path) {
        return false;
    }

    // Match lintr's `lax = TRUE` heuristic: require a separator followed by
    // at least two valid characters, while still allowing arbitrary Unicode.
    let mut component_count = 0;
    let mut has_long_component = false;
    for component in path
        .split(['/', '\\'])
        .filter(|component| !component.is_empty())
    {
        if !is_valid_component(component, component_count == 0) {
            return false;
        }

        if component_count > 0 && component.chars().count() >= 2 {
            has_long_component = true;
        }

        component_count += 1;
    }

    component_count >= 2 && has_long_component
}

fn is_skipped_context(string: &RStringValue) -> bool {
    // Walk all ancestors so a string nested in `paste0()` inside `grep()` is
    // still recognized as part of a regular-expression argument.
    for ancestor in string.syntax().ancestors() {
        if let Some(argument) = RArgument::cast(ancestor.clone())
            && argument
                .name_clause()
                .and_then(|clause| clause.name().ok())
                .is_some_and(|name| {
                    SKIPPED_ARGUMENT_NAMES.contains(&name.to_trimmed_string().as_str())
                })
        {
            return true;
        }

        if let Some(call) = RCall::cast(ancestor)
            && let Ok(function) = call.function()
            && SKIPPED_FUNCTIONS.contains(&get_function_name(function).as_str())
        {
            return true;
        }
    }

    false
}

fn has_url_scheme(path: &str) -> bool {
    let Some((scheme, _)) = path.split_once("://") else {
        return false;
    };
    !scheme.is_empty()
        && scheme
            .chars()
            .all(|character| character.is_ascii_alphabetic())
}

fn is_valid_component(component: &str, first: bool) -> bool {
    if first
        && component.len() == 2
        && component.ends_with(':')
        && component.starts_with(|character: char| character.is_ascii_alphabetic())
    {
        return true;
    }

    component.chars().all(|character| {
        !character.is_control()
            && !matches!(
                character,
                '*' | '?' | '"' | '<' | '>' | '|' | ':' | '/' | '\\'
            )
    })
}
