use crate::diagnostic::*;
use crate::utils::{
    get_arg_by_name_then_position, get_function_name, get_unnamed_args, node_contains_comments,
};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for usage of `class(...) == "some_class"` and
/// `class(...) %in% "some_class"`. The only cases that are flagged (and
/// potentially fixed) are cases that:
///
/// - happen in the condition part of an `if ()` statement or of a `while ()`
///   statement,
/// - and are not nested in other calls.
///
/// For example, `if (class(x) == "foo")` would be reported, but not
/// `if (my_function(class(x) == "foo"))`.
///
/// ## Why is this bad?
///
/// An R object can have several classes. Therefore,
/// `class(...) == "some_class"` would return a logical vector with as many
/// values as the object has classes, which is rarely desirable.
///
/// It is better to use `inherits(..., "some_class")` instead. `inherits()`
/// checks whether any of the object's classes match the desired class.
///
/// The same rationale applies to `class(...) %in% "some_class"`.
///
/// ## Example
///
/// ```r
/// x <- lm(drat ~ mpg, mtcars)
/// class(x) <- c("my_class", class(x))
///
/// if (class(x) == "lm") {
///   # <do something>
/// }
/// ```
///
/// Use instead:
/// ```r
/// x <- lm(drat ~ mpg, mtcars)
/// class(x) <- c("my_class", class(x))
///
/// if (inherits(x, "lm")) {
///   # <do something>
/// }
/// ```
///
/// ## References
///
/// See `?inherits`
pub fn sprintf(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let function_name = get_function_name(function);

    if function_name != "sprintf" {
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

    // Constant string, e.g. `sprintf("abc")`
    if !fmt_text.contains("%") {
        let range = ast.syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(
            ViolationData::new(
                "sprintf".to_string(),
                "`sprintf()` without special characters is useless.".to_string(),
                Some("Use directly the input of `sprintf()` instead.".to_string()),
            ),
            range,
            Fix {
                content: format!("{}", fmt_text),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        );
        return Ok(Some(diagnostic));
    };

    // Unknown characters following `%`, e.g. `sprintf("%y", "abc")`
    let invalid_percents = find_invalid_percent(&fmt_text);
    if invalid_percents.len() > 0 {
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

    let specs = parse_format_specifiers(&fmt_text);
    let dots = get_unnamed_args(&args);
    let len_dots = if fmt.name_clause().is_some() {
        dots.len()
    } else {
        dots.len() - 1
    };

    // If any specifier uses positional references, find max position
    // Otherwise, count the number of specifiers
    let expected_args = if specs.iter().any(|s| s.position.is_some()) {
        specs.iter().filter_map(|s| s.position).max().unwrap_or(0)
    } else {
        specs.len()
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

pub static SPRINTF_SPECIAL_CHARS: &[&str] = &[
    "d", "i", "o", "x", "X", "f", "e", "E", "g", "G", "a", "A", "s", "%", "m", ".", "n", "-", "+",
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "#",
];

struct FormatSpec {
    position: Option<usize>, // Some(1) for %1$s, None for %s
}

fn find_invalid_percent(s: &str) -> Vec<usize> {
    let mut invalid = Vec::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, &ch) in chars.iter().enumerate() {
        if ch == '%' {
            // Skip whitespace after %
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            // Check if next non-whitespace belongs to the list of special characters
            if j >= chars.len() || !SPRINTF_SPECIAL_CHARS.contains(&chars[j].to_string().as_str()) {
                invalid.push(i);
            }
        }
    }
    invalid
}

// This is more complicated than just checking the allowed characters after
// "%" because it is possible to add an index after "%" to refer to the argument
// position. For example, this is valid although the number of special characters
// and the number of arguments are not the same:
//
// sprintf('hello %1$s %1$s %2$d', x, y)
//
fn parse_format_specifiers(s: &str) -> Vec<FormatSpec> {
    let mut specs = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '%' {
            i += 1;
            // Skip whitespace
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }

            if i >= chars.len() {
                break;
            }

            // Check for %% (literal %)
            if chars[i] == '%' {
                i += 1;
                continue;
            }

            // Parse optional position (e.g., "1$" in "%1$s")
            let mut position = None;
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i < chars.len() && chars[i] == '$' {
                if let Ok(pos) = chars[start..i].iter().collect::<String>().parse::<usize>() {
                    position = Some(pos);
                    i += 1; // Skip the '$'
                }
            } else {
                i = start; // Reset, wasn't a position specifier
            }

            // Skip flags, width, precision (-, +, 0-9, ., #)
            while i < chars.len()
                && (chars[i] == '-'
                    || chars[i] == '+'
                    || chars[i] == '#'
                    || chars[i] == '0'
                    || chars[i] == '.'
                    || chars[i].is_ascii_digit())
            {
                i += 1;
            }

            // Check if we have a valid type specifier
            if i < chars.len() && SPRINTF_SPECIAL_CHARS.contains(&chars[i].to_string().as_str()) {
                specs.push(FormatSpec { position });
            }

            i += 1;
        } else {
            i += 1;
        }
    }
    specs
}
