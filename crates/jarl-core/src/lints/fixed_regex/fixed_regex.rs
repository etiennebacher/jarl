use crate::diagnostic::*;
use crate::unwrap_or_return_none;
use crate::utils::drop_arg_by_name_or_position;
use crate::utils::{get_arg_by_name_then_position, get_function_name, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct FixedRegex;

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
    fn name(&self) -> String {
        "fixed_regex".to_string()
    }
    fn body(&self) -> String {
        "Pattern contains no regex special characters but `fixed = TRUE` is not set.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Add `fixed = TRUE` for better performance.".to_string())
    }
}

pub fn fixed_regex(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let fn_name = get_function_name(function);
    let args = ast.arguments()?.items();

    // Determine the position of the 'fixed' argument based on the function
    let fixed_position = match fn_name.as_str() {
        "grep" | "gsub" | "sub" => 6,
        "regexpr" | "gregexpr" | "regexec" | "grepl" => 5,
        _ => return Ok(None),
    };

    // Check if fixed is already set to TRUE
    if let Some(fixed_arg) = get_arg_by_name_then_position(&args, "fixed", fixed_position)
        && let Some(value) = fixed_arg.value()
        && value.syntax().text_trimmed() == "TRUE"
    {
        // fixed = TRUE is already set, no need to lint
        return Ok(None);
    }

    // Check if ignore.case is set to TRUE (implies regex interpretation)
    let ignore_case_position = match fn_name.as_str() {
        "gsub" | "sub" => 4,
        "regexpr" | "gregexpr" | "regexec" | "grep" | "grepl" => 3,
        _ => return Ok(None),
    };
    if let Some(ignore_case_arg) =
        get_arg_by_name_then_position(&args, "ignore.case", ignore_case_position)
        && let Some(value) = ignore_case_arg.value()
        && value.syntax().text_trimmed() == "TRUE"
    {
        // ignore.case = TRUE implies regex interpretation is needed
        return Ok(None);
    }

    // Get the pattern argument (first argument for all functions)
    let pattern_arg = unwrap_or_return_none!(get_arg_by_name_then_position(&args, "pattern", 1));
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

    // Pattern is fixed but fixed = TRUE is not set
    // Build the fix by adding fixed = TRUE to the arguments or changing the value
    // of fixed = FALSE.
    let args_text = if let Some(fixed_arg) =
        get_arg_by_name_then_position(&args, "fixed", fixed_position)
        && let Some(value) = fixed_arg.value()
        && value.syntax().text_trimmed() == "FALSE"
    {
        unwrap_or_return_none!(drop_arg_by_name_or_position(&args, "fixed", fixed_position))
            .into_iter()
            .map(|arg| arg.syntax().text_trimmed().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        args.into_iter()
            .filter_map(|arg| arg.ok())
            .map(|arg| arg.syntax().text_trimmed().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };

    let fixed_content = format!("{}({}, fixed = TRUE)", fn_name, args_text);

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        FixedRegex,
        range,
        Fix {
            content: fixed_content,
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}

/// Check if a pattern string contains no unescaped regex special characters
fn is_fixed_pattern(pattern: &str) -> bool {
    const REGEX_CHARS: &[char] = &['.', '*', '+', '?', '[', '{', '(', ')', '|', '^', '$', '\\'];
    let chars = pattern.chars().peekable();

    for c in chars {
        // Unescaped character - check if it's a regex metacharacter
        if REGEX_CHARS.contains(&c) {
            return false;
        } else {
            continue;
        }
    }

    true
}
