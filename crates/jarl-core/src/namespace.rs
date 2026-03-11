//! Shared NAMESPACE file parsing utilities.
//!
//! Parses R package NAMESPACE files to extract exported function names.
//! Used by both the `unused_function` rule (for the user's own package)
//! and the `PackageCache` (for installed external packages).

use regex::Regex;
use std::collections::HashSet;

/// Extract the directive arguments from a NAMESPACE line.
///
/// Finds `directive(...)` in the line and returns the parenthesized content.
/// Handles lines where the directive is preceded by an `if (...)` guard.
fn extract_directive<'a>(line: &'a str, directive: &str) -> Option<&'a str> {
    let dir_with_paren = format!("{directive}(");
    let start = line.find(&dir_with_paren)?;
    let args_start = start + dir_with_paren.len();

    // Make sure the directive is not part of a longer word
    if start > 0 {
        let prev = line.as_bytes()[start - 1];
        if prev.is_ascii_alphanumeric() || prev == b'_' {
            return None;
        }
    }

    let rest = &line[args_start..];
    let end = rest.rfind(')')?;
    Some(&rest[..end])
}

/// Join multi-line directives into single strings.
///
/// NAMESPACE files can have directives that span multiple lines, e.g.:
/// ```text
/// export(foo, bar,
///        baz, qux)
/// ```
/// This function joins such lines by tracking unmatched parentheses.
fn join_continuation_lines(content: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut depth: i32 = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            if depth == 0 {
                statements.push(trimmed.to_string());
            }
            continue;
        }

        if depth > 0 {
            current.push(' ');
            current.push_str(trimmed);
        } else {
            current = trimmed.to_string();
        }

        for ch in trimmed.chars() {
            match ch {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
        }

        if depth <= 0 {
            statements.push(std::mem::take(&mut current));
            depth = 0;
        }
    }

    // Push any remaining incomplete directive
    if !current.is_empty() {
        statements.push(current);
    }

    statements
}

/// Parse a NAMESPACE file and return the set of exported function names.
///
/// Handles `export(name)`, `exportPattern(regex)`, `S3method(generic, class)`,
/// `exportMethods()`, and `exportClasses()` directives.
///
/// For `exportPattern`, the regex is compiled and matched against `all_names`
/// to expand it into concrete names. Pass an empty slice to skip pattern
/// expansion (suitable for external packages where we don't have the full
/// object list).
pub fn parse_namespace_exports(content: &str, all_names: &[&str]) -> HashSet<String> {
    let mut exports = HashSet::new();

    // Join multi-line directives: a line starting a directive (e.g. `export(`)
    // without a closing `)` continues on subsequent lines.
    let statements = join_continuation_lines(content);

    for statement in &statements {
        let trimmed = statement.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        for directive in [
            "export",
            "exportPattern",
            "S3method",
            "exportMethods",
            "exportClasses",
        ] {
            if let Some(inner) = extract_directive(trimmed, directive) {
                match directive {
                    "export" => {
                        for name in inner.split(',') {
                            let name = name.trim().trim_matches('"').trim_matches('\'');
                            if !name.is_empty() {
                                exports.insert(name.to_string());
                            }
                        }
                    }
                    "exportPattern" => {
                        let pattern = inner.trim().trim_matches('"').trim_matches('\'');
                        if let Ok(re) = Regex::new(pattern) {
                            for &name in all_names {
                                if re.is_match(name) {
                                    exports.insert(name.to_string());
                                }
                            }
                        }
                    }
                    "S3method" => {
                        let parts: Vec<&str> = inner.splitn(4, ',').collect();
                        if parts.len() >= 2 {
                            let raw_generic = parts[0].trim().trim_matches('"').trim_matches('\'');
                            let class = parts[1].trim().trim_matches('"').trim_matches('\'');

                            let generic = raw_generic
                                .rsplit_once("::")
                                .map(|(_, g)| g)
                                .unwrap_or(raw_generic);

                            if parts.len() >= 3 {
                                let method_fn =
                                    parts[2].trim().trim_matches('"').trim_matches('\'');
                                if !method_fn.is_empty() {
                                    exports.insert(method_fn.to_string());
                                }
                            } else {
                                exports.insert(format!("{generic}.{class}"));
                            }
                        }
                    }
                    "exportMethods" | "exportClasses" => {
                        for name in inner.split(',') {
                            let name = name.trim().trim_matches('"').trim_matches('\'');
                            if !name.is_empty() {
                                exports.insert(name.to_string());
                            }
                        }
                    }
                    _ => {}
                }
                break;
            }
        }
    }

    exports
}
