//! Shared NAMESPACE file parsing utilities.
//!
//! Parses R package NAMESPACE files to extract exported function names.
//! Used by both the `unused_function` rule (for the user's own package)
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

                            // Add the generic name so that packages providing
                            // S3 methods for a generic (e.g. tidypolars
                            // providing filter.polars_data_frame) are
                            // considered as candidates when resolving bare
                            // calls to that generic. This avoids false
                            // positives: without type information we can't
                            // know which method will dispatch at runtime.
                            exports.insert(generic.to_string());

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

/// S3 registration info extracted from `S3method()` directives.
#[derive(Clone, Debug, Default)]
pub struct S3Info {
    /// Generic names that appear in `S3method(generic, class)`.
    /// A function whose name is in this set defines an S3 generic.
    pub generics: HashSet<String>,
    /// Method names (`generic.class`) from `S3method(generic, class)`.
    /// A function whose name is in this set is an S3 method.
    pub methods: HashSet<String>,
}

/// Parse `S3method()` directives from a NAMESPACE file to extract
/// which function names are S3 generics and which are S3 methods.
pub fn parse_namespace_s3(content: &str) -> S3Info {
    let mut info = S3Info::default();
    let statements = join_continuation_lines(content);

    for statement in &statements {
        let trimmed = statement.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(inner) = extract_directive(trimmed, "S3method") {
            let parts: Vec<&str> = inner.splitn(4, ',').collect();
            if parts.len() >= 2 {
                let raw_generic = parts[0].trim().trim_matches('"').trim_matches('\'');
                let class = parts[1].trim().trim_matches('"').trim_matches('\'');

                let generic = raw_generic
                    .rsplit_once("::")
                    .map(|(_, g)| g)
                    .unwrap_or(raw_generic);

                info.generics.insert(generic.to_string());

                if parts.len() >= 3 {
                    let method_fn = parts[2].trim().trim_matches('"').trim_matches('\'');
                    if !method_fn.is_empty() {
                        info.methods.insert(method_fn.to_string());
                    }
                } else {
                    info.methods.insert(format!("{generic}.{class}"));
                }
            }
        }
    }

    info
}

/// Result of parsing `import()` and `importFrom()` directives from a
/// package's own NAMESPACE file.
#[derive(Debug, Default)]
pub struct NamespaceImports {
    /// Direct function→package mappings from `importFrom(pkg, fn1, fn2, ...)`.
    pub import_from: HashMap<String, String>,
    /// Packages imported wholesale via `import(pkg)`.
    pub blanket_imports: Vec<String>,
}

/// Parse a NAMESPACE file for `importFrom()` and `import()` directives.
///
/// - `importFrom(dplyr, filter, select)` → maps `filter` and `select` to `"dplyr"`
/// - `import(rlang)` → adds `"rlang"` to `blanket_imports`
pub fn parse_namespace_imports(content: &str) -> NamespaceImports {
    let mut result = NamespaceImports::default();
    let statements = join_continuation_lines(content);

    for statement in &statements {
        let trimmed = statement.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(inner) = extract_directive(trimmed, "importFrom") {
            let parts: Vec<&str> = inner
                .split(',')
                .map(|s| s.trim().trim_matches('"').trim_matches('\''))
                .collect();
            if parts.len() >= 2 {
                let pkg = parts[0];
                for &fn_name in &parts[1..] {
                    if !fn_name.is_empty() {
                        result
                            .import_from
                            .insert(fn_name.to_string(), pkg.to_string());
                    }
                }
            }
        } else if let Some(inner) = extract_directive(trimmed, "import") {
            for pkg in inner.split(',') {
                let pkg = pkg.trim().trim_matches('"').trim_matches('\'');
                if !pkg.is_empty() && !result.blanket_imports.contains(&pkg.to_string()) {
                    result.blanket_imports.push(pkg.to_string());
                }
            }
        }
    }

    result
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
        let result = parse_namespace_imports(ns);
        assert_eq!(result.import_from.get("filter").unwrap(), "dplyr");
        assert_eq!(result.import_from.get("select").unwrap(), "dplyr");
        assert_eq!(result.import_from.get("mutate").unwrap(), "dplyr");
        assert_eq!(result.import_from.get("!!").unwrap(), "rlang");
        assert!(result.blanket_imports.is_empty());
    }

    #[test]
    fn test_parse_blanket_import() {
        let ns = "import(rlang)\nimport(dplyr, tidyr)\n";
        let result = parse_namespace_imports(ns);
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
        let result = parse_namespace_imports(ns);
        assert_eq!(result.import_from.get("filter").unwrap(), "dplyr");
        assert_eq!(result.blanket_imports, vec!["rlang"]);
    }

    #[test]
    fn test_parse_multiline_import_from() {
        let ns = "importFrom(dplyr,\n  filter,\n  select)\n";
        let result = parse_namespace_imports(ns);
        assert_eq!(result.import_from.get("filter").unwrap(), "dplyr");
        assert_eq!(result.import_from.get("select").unwrap(), "dplyr");
    }

    #[test]
    fn test_no_duplicate_blanket_imports() {
        let ns = "import(dplyr)\nimport(dplyr)\n";
        let result = parse_namespace_imports(ns);
        assert_eq!(result.blanket_imports, vec!["dplyr"]);
    }
}
