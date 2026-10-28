//! Shared NAMESPACE file parsing utilities.
//!
//! Parses R package NAMESPACE files into a [`Namespace`].
//! Used by the `unused_function` and `unused_object` rules (for the user's own package)
//! and the `PackageCache` (for installed external packages).

use regex::Regex;
use std::collections::{HashMap, HashSet};

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

/// A parsed NAMESPACE file.
///
/// Parsed once per package; callers then resolve it against whichever set of
/// defined names they care about (top-level functions, all top-level objects,
/// ...) with [`Namespace::exported`].
#[derive(Clone, Debug, Default)]
pub struct Namespace {
    /// Names listed literally: `export()`, `exportMethods()`,
    /// `exportClasses()` and the generic/method names of `S3method()`.
    pub exports: HashSet<String>,
    /// Compiled `exportPattern()` regexes, not yet matched against any names.
    export_patterns: Vec<Regex>,
    /// Direct function→package mappings from `importFrom(pkg, fn1, fn2, ...)`.
    pub import_from: HashMap<String, String>,
    /// Packages imported wholesale via `import(pkg)`.
    pub blanket_imports: Vec<String>,
}

fn unquote(s: &str) -> &str {
    s.trim().trim_matches('"').trim_matches('\'')
}

impl Namespace {
    /// Parse the content of a NAMESPACE file.
    ///
    /// Handles `export`, `exportPattern`, `S3method`, `exportMethods`,
    /// `exportClasses`, `import` and `importFrom` directives.
    pub fn parse(content: &str) -> Self {
        let mut ns = Namespace::default();

        for statement in join_continuation_lines(content) {
            let trimmed = statement.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // `importFrom` must be tried before `import`, and `exportPattern`
            // etc. are disjoint from both.
            if let Some(inner) = extract_directive(trimmed, "importFrom") {
                let parts: Vec<&str> = inner.split(',').map(unquote).collect();
                if parts.len() >= 2 {
                    for &fn_name in &parts[1..] {
                        if !fn_name.is_empty() {
                            ns.import_from
                                .insert(fn_name.to_string(), parts[0].to_string());
                        }
                    }
                }
            } else if let Some(inner) = extract_directive(trimmed, "import") {
                for pkg in inner.split(',').map(unquote) {
                    if !pkg.is_empty() && !ns.blanket_imports.iter().any(|p| p == pkg) {
                        ns.blanket_imports.push(pkg.to_string());
                    }
                }
            } else if let Some(inner) = extract_directive(trimmed, "exportPattern") {
                if let Ok(re) = Regex::new(unquote(inner)) {
                    ns.export_patterns.push(re);
                }
            } else if let Some(inner) = extract_directive(trimmed, "S3method") {
                ns.add_s3_method(inner);
            } else if let Some(inner) = extract_directive(trimmed, "export")
                .or_else(|| extract_directive(trimmed, "exportMethods"))
                .or_else(|| extract_directive(trimmed, "exportClasses"))
            {
                for name in inner.split(',').map(unquote) {
                    if !name.is_empty() {
                        ns.exports.insert(name.to_string());
                    }
                }
            }
        }

        ns
    }

    fn add_s3_method(&mut self, inner: &str) {
        let parts: Vec<&str> = inner.splitn(4, ',').collect();
        if parts.len() < 2 {
            return;
        }
        let raw_generic = unquote(parts[0]);
        let class = unquote(parts[1]);

        let generic = raw_generic
            .rsplit_once("::")
            .map(|(_, g)| g)
            .unwrap_or(raw_generic);

        // Add the generic name so that packages providing S3 methods for a
        // generic (e.g. tidypolars providing filter.polars_data_frame) are
        // considered as candidates when resolving bare calls to that generic.
        // This avoids false positives: without type information we can't know
        // which method will dispatch at runtime.
        self.exports.insert(generic.to_string());

        if parts.len() >= 3 {
            let method_fn = unquote(parts[2]);
            if !method_fn.is_empty() {
                self.exports.insert(method_fn.to_string());
            }
        } else {
            self.exports.insert(format!("{generic}.{class}"));
        }
    }

    /// The literal exports plus every name in `defined` matched by an
    /// `exportPattern()`. Pass an empty iterator to skip pattern expansion
    /// (suitable for external packages where we don't have the full object
    /// list).
    pub fn exported<'a>(&self, defined: impl IntoIterator<Item = &'a str>) -> HashSet<String> {
        let mut exports = self.exports.clone();
        if !self.export_patterns.is_empty() {
            for name in defined {
                if self.export_patterns.iter().any(|re| re.is_match(name)) {
                    exports.insert(name.to_string());
                }
            }
        }
        exports
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_import_from() {
        let ns = r#"
importFrom(dplyr, filter, select, mutate)
importFrom(rlang, "!!")
"#;
        let result = Namespace::parse(ns);
        assert_eq!(result.import_from.get("filter").unwrap(), "dplyr");
        assert_eq!(result.import_from.get("select").unwrap(), "dplyr");
        assert_eq!(result.import_from.get("mutate").unwrap(), "dplyr");
        assert_eq!(result.import_from.get("!!").unwrap(), "rlang");
        assert!(result.blanket_imports.is_empty());
    }

    #[test]
    fn test_parse_blanket_import() {
        let ns = "import(rlang)\nimport(dplyr, tidyr)\n";
        let result = Namespace::parse(ns);
        assert!(result.import_from.is_empty());
        assert_eq!(result.blanket_imports, vec!["rlang", "dplyr", "tidyr"]);
    }

    #[test]
    fn test_parse_mixed_imports() {
        let ns = r#"
import(rlang)
importFrom(dplyr, filter)
export(my_fn)
"#;
        let result = Namespace::parse(ns);
        assert_eq!(result.import_from.get("filter").unwrap(), "dplyr");
        assert_eq!(result.blanket_imports, vec!["rlang"]);
    }

    #[test]
    fn test_parse_multiline_import_from() {
        let ns = "importFrom(dplyr,\n  filter,\n  select)\n";
        let result = Namespace::parse(ns);
        assert_eq!(result.import_from.get("filter").unwrap(), "dplyr");
        assert_eq!(result.import_from.get("select").unwrap(), "dplyr");
    }

    #[test]
    fn test_s3method_without_class_is_ignored() {
        let ns = Namespace::parse("S3method(print)\n");
        assert!(ns.exports.is_empty());
    }

    #[test]
    fn test_no_duplicate_blanket_imports() {
        let ns = "import(dplyr)\nimport(dplyr)\n";
        let result = Namespace::parse(ns);
        assert_eq!(result.blanket_imports, vec!["dplyr"]);
    }
}
