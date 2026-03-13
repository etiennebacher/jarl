//
// Adapted from Ark
// https://github.com/posit-dev/ark/blob/main/crates/ark/src/lsp/inputs/package_description.rs
// 7f9ea95d367712eb40b1669cf317c7a8a71e779b
//
// MIT License - Posit PBC

use anyhow;
use std::collections::HashMap;

/// Simple parser for R version requirements from DESCRIPTION files
pub struct Description;

impl Description {
    /// Extract package names from the specified DESCRIPTION fields.
    ///
    /// `fields` should be a slice of field names, e.g.
    /// `&["Depends", "Imports"]` or `&["Depends", "Imports", "Suggests"]`.
    ///
    /// Returns package names (excluding `R` itself) in the order they appear.
    pub fn get_package_deps(contents: &str, what: &[&str]) -> Vec<String> {
        let parsed = parse_dcf(contents);
        let mut packages = Vec::new();

        for field_name in what {
            if let Some(value) = parsed.get(*field_name) {
                for dep in value.split(',') {
                    // Strip version constraints: "dplyr (>= 1.0.0)" → "dplyr"
                    let name = dep.split('(').next().unwrap_or("").trim();
                    if !name.is_empty() && name != "R" && !packages.contains(&name.to_string()) {
                        packages.push(name.to_string());
                    }
                }
            }
        }

        packages
    }

    /// Extract R version requirements from the Depends field of a DESCRIPTION file
    ///
    /// Returns a vector of version strings found in R dependencies.
    /// Examples:
    /// - "Depends: R (>= 4.3.0)" -> ["4.3.0"]
    /// - "Depends: R" -> []
    /// - No Depends field -> []
    pub fn get_depend_r_version(contents: &str) -> anyhow::Result<Vec<String>> {
        let fields = parse_dcf(contents);

        let r_versions = fields
            .get("Depends")
            .map(|deps| {
                deps.split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty() && s.starts_with("R "))
                    .filter_map(extract_version_from_dependency)
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        Ok(r_versions)
    }
}

/// Extract version number from an R dependency string like "R (>= 4.3.0)"
fn extract_version_from_dependency(dep: &str) -> Option<String> {
    // Look for version requirement in parentheses
    if let Some(start) = dep.find('(')
        && let Some(end) = dep.find(')')
    {
        let version_part = &dep[start + 1..end];
        // Remove >= operator and extract just the version number
        let version = version_part.replace(">=", "").trim().to_string();

        if !version.is_empty() {
            return Some(version);
        }
    }

    // R dependency exists but no version specified
    unreachable!("DESCRIPTION cannot have 'R' without version in Depends field.")
}

/// Parse a DCF (Debian Control File) format string into a key-value map
/// Minimal implementation focused on extracting the Depends field
fn parse_dcf(input: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    let mut current_key: Option<String> = None;
    let mut current_value = String::new();

    for line in input.lines() {
        // Indented line: continuation of previous field
        if line.starts_with(char::is_whitespace) {
            current_value.push_str(line.trim());
            current_value.push(' ');
            continue;
        }

        // New field: contains a colon and doesn't start with whitespace
        if !line.is_empty() && line.contains(':') {
            // Save previous field if exists
            if let Some(key) = current_key.take() {
                fields.insert(key, current_value.trim().to_string());
            }

            // Parse new field
            let colon_pos = line.find(':').unwrap();
            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim();

            current_key = Some(key);
            current_value = String::from(value);

            if !current_value.is_empty() {
                current_value.push(' ');
            }
        }
    }

    // Save the last field
    if let Some(key) = current_key {
        fields.insert(key, current_value.trim().to_string());
    }

    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_depends_field() {
        let description = r#"
Package: mypackage
Version: 1.0.0
Title: My Package
"#;
        let result = Description::get_depend_r_version(description).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_depends_without_r() {
        let description = r#"
Package: mypackage
Version: 1.0.0
Depends: dplyr, ggplot2
"#;
        let result = Description::get_depend_r_version(description).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_depends_r_with_version() {
        let description = r#"
Package: mypackage
Version: 1.0.0
Depends: R (>= 4.3.0), dplyr
"#;
        let result = Description::get_depend_r_version(description).unwrap();
        assert_eq!(result, vec!["4.3.0"]);

        let description = r#"
Package: mypackage
Version: 1.0.0
Depends: dplyr, R (>= 4.3.0)
"#;
        let result = Description::get_depend_r_version(description).unwrap();
        assert_eq!(result, vec!["4.3.0"]);
    }

    #[test]
    fn test_depends_r_with_version_operator() {
        let description = r#"
Package: mypackage
Version: 1.0.0
Depends: R (>= 4.2.0)
"#;
        let result = Description::get_depend_r_version(description).unwrap();
        assert_eq!(result, vec!["4.2.0"]);
    }

    #[test]
    fn test_depends_multiline() {
        let description = r#"
Package: mypackage
Version: 1.0.0
Depends: R (>= 4.3.0),
    dplyr,
    ggplot2
"#;
        let result = Description::get_depend_r_version(description).unwrap();
        assert_eq!(result, vec!["4.3.0"]);
    }

    #[test]
    fn test_depends_with_spacing() {
        let description = r#"
Package: mypackage
Version: 1.0.0
Depends: R ( >= 4.3.0 ), dplyr
"#;
        let result = Description::get_depend_r_version(description).unwrap();
        assert_eq!(result, vec!["4.3.0"]);
    }

    #[test]
    fn test_get_package_deps_imports_and_depends() {
        let description = r#"
Package: mypackage
Version: 1.0.0
Depends: R (>= 4.3.0), rlang
Imports: dplyr (>= 1.0.0), tidyr, ggplot2
"#;
        let result = Description::get_package_deps(description, &["Depends", "Imports"]);
        assert_eq!(result, vec!["rlang", "dplyr", "tidyr", "ggplot2"]);

        let result = Description::get_package_deps(description, &["Depends"]);
        assert_eq!(result, vec!["rlang"]);
    }

    #[test]
    fn test_get_package_deps_no_deps() {
        let description = r#"
Package: mypackage
Version: 1.0.0
"#;
        let result = Description::get_package_deps(description, &["Depends", "Imports"]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_package_deps_excludes_r() {
        let description = r#"
Package: mypackage
Depends: R (>= 4.0.0)
Imports: dplyr
"#;
        let result = Description::get_package_deps(description, &["Depends", "Imports"]);
        assert_eq!(result, vec!["dplyr"]);
    }

    #[test]
    fn test_get_package_deps_no_duplicates() {
        let description = r#"
Package: mypackage
Depends: dplyr
Imports: dplyr, tidyr
"#;
        let result = Description::get_package_deps(description, &["Depends", "Imports"]);
        assert_eq!(result, vec!["dplyr", "tidyr"]);
    }
}
