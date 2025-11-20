//! Utility functions for the Jarl LSP server

use std::path::Path;

use air_workspace::resolve::PathResolver;
use jarl_core::discovery::DEFAULT_EXCLUDE_PATTERNS;
use jarl_core::settings::Settings;

/// Check if a path string matches an exclusion pattern
///
/// Patterns can be:
/// - Directory patterns ending with `/` (e.g., "renv/")
/// - Exact filename matches (e.g., "cpp11.R")
/// - Glob patterns with `*` wildcards (e.g., "import-standalone-*.R")
pub fn matches_pattern(path: &str, pattern: &str) -> bool {
    // Normalize path separators to forward slashes for consistent matching
    let normalized_path = path.replace('\\', "/");

    if pattern.ends_with('/') {
        // Directory pattern - check if path contains this directory
        let dir_pattern = pattern.trim_end_matches('/');

        // Match if the directory appears as a path component
        // e.g., "renv/" should match "path/to/renv/file.R" but not "path/to/myrenv/file.R"
        for component in normalized_path.split('/') {
            if component == dir_pattern {
                return true;
            }
        }
        false
    } else if pattern.contains('*') {
        // Glob pattern - use simple glob matching
        let filename = normalized_path
            .split('/')
            .next_back()
            .unwrap_or(&normalized_path);

        // Split pattern by '*' to get literal parts
        let parts: Vec<&str> = pattern.split('*').collect();

        if parts.is_empty() {
            return false;
        }

        // Check if filename starts with first part
        if !filename.starts_with(parts[0]) {
            return false;
        }

        // Check if filename ends with last part (if there's more than one part)
        if parts.len() > 1 {
            let last = parts[parts.len() - 1];
            if !filename.ends_with(last) {
                return false;
            }
        }

        // For patterns with multiple wildcards, check that all parts appear in order
        let mut pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if i == 0 {
                pos = part.len();
                continue;
            }

            if let Some(found) = filename[pos..].find(part) {
                pos += found + part.len();
            } else {
                return false;
            }
        }

        true
    } else {
        // Exact filename match - check if the filename component matches
        normalized_path
            .split('/')
            .next_back()
            .map(|filename| filename == pattern)
            .unwrap_or(false)
    }
}

/// Check if a file should be excluded based on patterns
///
/// Returns `true` if the file path matches any of the provided exclusion patterns.
pub fn should_exclude_file(file_path: &Path, patterns: &[&str]) -> bool {
    let path_str = file_path.to_string_lossy();

    for pattern in patterns {
        if matches_pattern(&path_str, pattern) {
            tracing::debug!(
                "File {:?} matches exclusion pattern '{}'",
                file_path,
                pattern
            );
            return true;
        }
    }

    false
}

/// Check if a file should be excluded based on settings from jarl.toml
///
/// This function checks both the `default-exclude` option (which is `true` by default)
/// and any custom `exclude` patterns specified in the jarl.toml configuration.
///
/// # Arguments
/// * `file_path` - The path to the file to check
/// * `resolver` - The path resolver containing the discovered settings
///
/// # Returns
/// `true` if the file should be excluded from linting, `false` otherwise
pub fn should_exclude_file_based_on_settings(
    file_path: &Path,
    resolver: &PathResolver<Settings>,
) -> bool {
    // Get the first settings item (the one applicable to this file)
    let Some(settings_item) = resolver.items().first() else {
        // No settings found, don't exclude
        return false;
    };

    let settings = settings_item.value();

    // Check if default_exclude is enabled (true by default)
    let use_default_exclude = settings.linter.default_exclude.unwrap_or(true);

    let mut patterns = Vec::new();

    if use_default_exclude {
        patterns.extend_from_slice(DEFAULT_EXCLUDE_PATTERNS);
    }

    // Add custom exclude patterns from jarl.toml
    if let Some(exclude_patterns) = &settings.linter.exclude {
        for pattern in exclude_patterns {
            patterns.push(pattern.as_str());
        }
    }

    // Check if the file matches any exclusion pattern
    if !patterns.is_empty() && should_exclude_file(file_path, &patterns) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_directory_pattern_basic() {
        assert!(matches_pattern("project/renv/activate.R", "renv/"));
        assert!(matches_pattern("renv/library/package.R", "renv/"));
        assert!(matches_pattern("path/to/renv/file.R", "renv/"));
    }

    #[test]
    fn test_directory_pattern_no_substring_match() {
        // Should not match directory name as substring
        assert!(!matches_pattern("project/myrenv/file.R", "renv/"));
        assert!(!matches_pattern("project/renvfoo/file.R", "renv/"));
        assert!(!matches_pattern("project/foorenv/file.R", "renv/"));
    }

    #[test]
    fn test_directory_pattern_git() {
        assert!(matches_pattern("project/.git/config", ".git/"));
        assert!(matches_pattern(".git/HEAD", ".git/"));
        assert!(matches_pattern("path/to/.git/objects/file", ".git/"));
    }

    #[test]
    fn test_exact_filename_match() {
        assert!(matches_pattern("project/src/cpp11.R", "cpp11.R"));
        assert!(matches_pattern("RcppExports.R", "RcppExports.R"));
        assert!(matches_pattern(
            "path/to/extendr-wrappers.R",
            "extendr-wrappers.R"
        ));
    }

    #[test]
    fn test_exact_filename_no_substring_match() {
        // Should not match as substring
        assert!(!matches_pattern("my-cpp11.R", "cpp11.R"));
        assert!(!matches_pattern("cpp11-extra.R", "cpp11.R"));
        assert!(!matches_pattern("path/to/mycpp11.R", "cpp11.R"));
    }

    #[test]
    fn test_glob_pattern_basic() {
        assert!(matches_pattern(
            "project/import-standalone-purrr.R",
            "import-standalone-*.R"
        ));
        assert!(matches_pattern(
            "import-standalone-test.R",
            "import-standalone-*.R"
        ));
        assert!(matches_pattern(
            "R/import-standalone-types.R",
            "import-standalone-*.R"
        ));
    }

    #[test]
    fn test_glob_pattern_no_match() {
        // Should not match without the prefix
        assert!(!matches_pattern(
            "standalone-test.R",
            "import-standalone-*.R"
        ));

        // Should not match with wrong extension
        assert!(!matches_pattern(
            "import-standalone-test.py",
            "import-standalone-*.R"
        ));

        // Should not match without the correct start
        assert!(!matches_pattern(
            "test-import-standalone.R",
            "import-standalone-*.R"
        ));
    }

    #[test]
    fn test_glob_pattern_empty_wildcard() {
        // Wildcard can match empty string
        assert!(matches_pattern(
            "import-standalone-.R",
            "import-standalone-*.R"
        ));
    }

    #[test]
    fn test_glob_pattern_multiple_wildcards() {
        assert!(matches_pattern("test-foo-bar.R", "test-*-*.R"));
        assert!(matches_pattern("test-a-b.R", "test-*-*.R"));
        assert!(!matches_pattern("test-foo.R", "test-*-*.R"));
    }

    #[test]
    fn test_windows_paths() {
        assert!(matches_pattern("project\\renv\\activate.R", "renv/"));
        assert!(matches_pattern("project\\src\\cpp11.R", "cpp11.R"));
        assert!(matches_pattern(
            "project\\import-standalone-test.R",
            "import-standalone-*.R"
        ));
    }

    #[test]
    fn test_mixed_separators() {
        assert!(matches_pattern("project/renv\\activate.R", "renv/"));
        assert!(matches_pattern("project\\path/to\\cpp11.R", "cpp11.R"));
    }

    #[test]
    fn test_should_exclude_file_single_pattern() {
        let path = PathBuf::from("project/renv/activate.R");
        assert!(should_exclude_file(&path, &["renv/"]));

        let path = PathBuf::from("project/src/main.R");
        assert!(!should_exclude_file(&path, &["renv/"]));
    }

    #[test]
    fn test_should_exclude_file_multiple_patterns() {
        let patterns = &["renv/", "cpp11.R", "import-standalone-*.R"];

        assert!(should_exclude_file(
            &PathBuf::from("renv/activate.R"),
            patterns
        ));
        assert!(should_exclude_file(&PathBuf::from("src/cpp11.R"), patterns));
        assert!(should_exclude_file(
            &PathBuf::from("import-standalone-test.R"),
            patterns
        ));
        assert!(!should_exclude_file(
            &PathBuf::from("src/my_file.R"),
            patterns
        ));
    }

    #[test]
    fn test_should_exclude_file_empty_patterns() {
        let path = PathBuf::from("project/renv/activate.R");
        assert!(!should_exclude_file(&path, &[]));
    }

    #[test]
    fn test_revdep_directory() {
        assert!(matches_pattern("project/revdep/check.R", "revdep/"));
        assert!(matches_pattern("revdep/README.md", "revdep/"));
        assert!(!matches_pattern("project/myrevdep/file.R", "revdep/"));
    }

    #[test]
    fn test_default_exclude_patterns() {
        use jarl_core::discovery::DEFAULT_EXCLUDE_PATTERNS;

        // Test all default exclusion patterns
        assert!(should_exclude_file(
            &PathBuf::from("project/.git/config"),
            DEFAULT_EXCLUDE_PATTERNS
        ));
        assert!(should_exclude_file(
            &PathBuf::from("renv/activate.R"),
            DEFAULT_EXCLUDE_PATTERNS
        ));
        assert!(should_exclude_file(
            &PathBuf::from("revdep/check.R"),
            DEFAULT_EXCLUDE_PATTERNS
        ));
        assert!(should_exclude_file(
            &PathBuf::from("src/cpp11.R"),
            DEFAULT_EXCLUDE_PATTERNS
        ));
        assert!(should_exclude_file(
            &PathBuf::from("R/RcppExports.R"),
            DEFAULT_EXCLUDE_PATTERNS
        ));
        assert!(should_exclude_file(
            &PathBuf::from("R/extendr-wrappers.R"),
            DEFAULT_EXCLUDE_PATTERNS
        ));
        assert!(should_exclude_file(
            &PathBuf::from("R/import-standalone-purrr.R"),
            DEFAULT_EXCLUDE_PATTERNS
        ));

        // Should not exclude normal files
        assert!(!should_exclude_file(
            &PathBuf::from("R/utils.R"),
            DEFAULT_EXCLUDE_PATTERNS
        ));
        assert!(!should_exclude_file(
            &PathBuf::from("src/main.R"),
            DEFAULT_EXCLUDE_PATTERNS
        ));
    }
}
