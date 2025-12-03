use crate::diagnostic::*;
use crate::utils::{
    get_arg_by_name_then_position, get_function_name, get_unnamed_args, node_contains_comments,
};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Multiple checks for `sprintf()`:
///
/// 1. checks whether the `fmt` argument is a constant string (in which case
///    `sprintf()` is not needed);
/// 2. checks whether there is a mismatch between the number of special characters
///    and the number of arguments;
/// 3. checks whether the `fmt` argument contains invalid special characters.
///
/// ## Why is this bad?
///
/// For 1, using `sprintf()` with a constant string, e.g. `sprintf("abc")`, is
/// useless and less readable. This has a safe fix that extracts the string.
///
/// For 2, a mismatch between the number of special characters and the number of
/// arguments would generate a runtime error or a warning:
///
/// - if the number of special characters > number of arguments, it errors, e.g.
///   `sprintf("%s %s", "a")`;
/// - otherwise, it warns, e.g. `sprintf("%s", "a", "b")`.
///
/// For 3, passing invalid special characters would error at runtime, e.g.
/// `sprintf("%y", "a")`.
///
/// Cases 2 and 3 do not have an automatic fix.
///
/// ## Example
///
/// ```r
/// sprintf("abc")
/// sprintf("%s %s", "a") # would error
/// sprintf("%y", "a") # would error
/// ```
///
/// Use instead:
/// ```r
/// "abc"
/// ```
///
/// ## References
///
/// See `?sprintf`
pub fn sprintf(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let function_name = get_function_name(function);

    if function_name != "sprintf" {
        return Ok(None);
    }

    // Don't know how to handle pipes for now.
    if let Some(prev_sibling) = ast.syntax().prev_sibling_or_token()
        && prev_sibling.kind() == RSyntaxKind::PIPE
    {
        return Ok(None);
    }

    let args = ast.arguments()?.items();

    let fmt = unwrap_or_return_none!(get_arg_by_name_then_position(&args, "fmt", 1));
    let fmt_value = unwrap_or_return_none!(fmt.value());
    let fmt_text = if let Some(x) = fmt_value.as_any_r_value()
        && let Some(x) = x.as_r_string_value()
    {
        x.to_trimmed_string()
    } else {
        return Ok(None);
    };

    // Parse format string once
    let parse_result = parse_sprintf_format(&fmt_text);

    // Check for invalid patterns first
    if !parse_result.invalid_positions.is_empty() {
        let range = ast.syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(
            ViolationData::new(
                "sprintf".to_string(),
                "`sprintf()` contains some invalid `%`.".to_string(),
                None,
            ),
            range,
            Fix::empty(),
        );
        return Ok(Some(diagnostic));
    }

    // Check if it's a constant string (no valid specifiers)
    if parse_result.n_unique_special_chars == 0 {
        let range = ast.syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(
            ViolationData::new(
                "sprintf".to_string(),
                "`sprintf()` without special characters is useless.".to_string(),
                Some("Use directly the input of `sprintf()` instead.".to_string()),
            ),
            range,
            Fix {
                content: parse_result.output_string,
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        );
        return Ok(Some(diagnostic));
    }

    let dots = get_unnamed_args(&args);
    let len_dots = if fmt.name_clause().is_some() {
        dots.len()
    } else {
        dots.len() - 1
    };

    // If any specifier uses positional references, use max position
    // Otherwise, sum up all arguments consumed (including * for width/precision)
    let expected_args = if parse_result.has_positional {
        parse_result.max_position
    } else {
        parse_result.all_args_consumed.iter().sum()
    };

    if expected_args != len_dots {
        let range = ast.syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(
            ViolationData::new(
                "sprintf".to_string(),
                "Mismatch between number of special characters and number of arguments."
                    .to_string(),
                Some(format!(
                    "Found {} special character(s) and {} argument(s).",
                    expected_args, len_dots
                )),
            ),
            range,
            Fix::empty(),
        );
        return Ok(Some(diagnostic));
    }

    Ok(None)
}

pub static SPRINTF_TYPE_CHARS: &[char] = &[
    'd', 'i', 'o', 'x', 'X', 'f', 'e', 'E', 'g', 'G', 'a', 'A', 's',
];

// Store all the necessary info regarding special characters starting with "%"
// in the `fmt` arg.
struct SprintfParseResult {
    // Count unique special chars, e.g. `'hello %1$s %1$s'` returns 1.
    n_unique_special_chars: usize,
    // Number of args consumed (1 + * for width + * for precision)
    all_args_consumed: Vec<usize>,
    // Count invalid special chars, e.g. `'hello %s %y'` returns 1.
    invalid_positions: Vec<usize>,
    // Check if any special char has an index, e.g. `'hello %s %1$s'` returns true.
    has_positional: bool,
    // Find the highest index, e.g. `'hello %1s %1$s %2$s'` returns 2.
    max_position: usize,
    // Output string: only here for the special case of "%%", which is converted
    // to "%" in the case of constant strings.
    output_string: String,
}

// Parse sprintf format string in one pass
// Handles:
// - %% (literal %)
// - %1$s (positional specifiers)
// - Invalid patterns
fn parse_sprintf_format(s: &str) -> SprintfParseResult {
    let mut n_unique_special_chars = 0;
    let mut all_args_consumed: Vec<usize> = vec![];
    let mut invalid_positions = Vec::new();
    let mut has_positional = false;
    let mut max_position = 0;
    let mut output_string = String::new();

    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '%' {
            let percent_pos = i;
            i += 1;

            // Skip whitespace
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }

            if i >= chars.len() {
                // % at end of string with no type
                invalid_positions.push(percent_pos);
                continue;
            }

            // Check for %% (literal %)
            if chars[i] == '%' {
                output_string.push('%');
                i += 1;
                continue;
            }

            let mut args_consumed = 1; // At least 1 for the value itself

            // Parse optional position (e.g., "1$" in "%1$s")
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i < chars.len() && chars[i] == '$' {
                if let Ok(pos) = chars[start..i].iter().collect::<String>().parse::<usize>() {
                    has_positional = true;
                    if pos > max_position {
                        max_position = pos;
                    }
                    i += 1; // Skip the '$'
                }
            } else {
                i = start; // Reset, wasn't a position specifier
            }

            // Skip flags (-, +, space, 0, #)
            while i < chars.len()
                && (chars[i] == '-'
                    || chars[i] == '+'
                    || chars[i] == ' '
                    || chars[i] == '#'
                    || chars[i] == '0')
            {
                i += 1;
            }

            // Parse width (can be * or digits)
            if i < chars.len() && chars[i] == '*' {
                args_consumed += 1; // * consumes an argument
                i += 1;
            } else {
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }

            // Parse precision (. followed by * or digits)
            if i < chars.len() && chars[i] == '.' {
                i += 1;
                if i < chars.len() && chars[i] == '*' {
                    args_consumed += 1; // .* consumes an argument
                    i += 1;
                } else {
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }

            // Check if we have a valid type specifier
            if i < chars.len() && SPRINTF_TYPE_CHARS.contains(&chars[i]) {
                n_unique_special_chars += 1;
                all_args_consumed.push(args_consumed);
                i += 1;
            } else {
                // Invalid format specifier
                invalid_positions.push(percent_pos);
            }
        } else {
            output_string.push(chars[i]);
            i += 1;
        }
    }

    SprintfParseResult {
        n_unique_special_chars,
        all_args_consumed,
        invalid_positions,
        has_positional,
        max_position,
        output_string,
    }
}
