//
// DCF field access delegates to `oak_package_metadata::dcf::Dcf`; the
// R-specific extraction (package dependency lists, R version constraints
// from `Depends`) lives here.

use anyhow;
use oak_package_metadata::dcf::Dcf;

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
        let parsed = Dcf::parse(contents);
        let mut packages = Vec::new();

        for field_name in what {
            if let Some(value) = parsed.get(field_name) {
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
        let fields = Dcf::parse(contents);

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
