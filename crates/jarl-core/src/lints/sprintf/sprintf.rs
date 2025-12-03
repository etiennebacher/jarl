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

    let valid_percents = find_valid_percent(&fmt_text);
    let dots = get_unnamed_args(&args);
    let len_dots = if fmt.name_clause().is_some() {
        dots.len()
    } else {
        dots.len() - 1
    };

    if valid_percents.len() != len_dots {
        let range = ast.syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(
            ViolationData::new(
                "sprintf".to_string(),
                "Mismatch between number of special characters and number of arguments."
                    .to_string(),
                Some(format!(
                    "Found {} special character(s) and {} argument(s).",
                    valid_percents.len(),
                    len_dots
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

fn find_valid_percent(s: &str) -> Vec<usize> {
    let mut valid = Vec::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, &ch) in chars.iter().enumerate() {
        if ch == '%' {
            // Skip whitespace after %
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            // Check if next non-whitespace belongs to the list of special characters
            if j <= chars.len() && SPRINTF_SPECIAL_CHARS.contains(&chars[j].to_string().as_str()) {
                valid.push(i);
            }
        }
    }
    valid
}
