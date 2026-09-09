//! Utility functions for the Jarl LSP server

use std::path::Path;

use air_workspace::resolve::PathResolver;
use jarl_core::discovery::{
    DEFAULT_EXCLUDE_PATTERNS, exclude_matcher, excluded_with_parents, include_matcher,
};
use jarl_core::settings::Settings;

/// Check if a file should be excluded based on settings from jarl.toml
///
/// Honours the `exclude`, `default-exclude` and `include` options. The matching
/// itself is delegated to `jarl_core::discovery` so that the LSP and
/// `jarl check` exclude exactly the same files.
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
    // Exclude patterns are anchored at the directory of the config they come
    // from, so the nearest config wins. `resolve()` misses the user-level config
    // directory, which is never an ancestor of the file, hence the fallback.
    let Some(settings_item) = resolver
        .resolve(file_path)
        .or_else(|| resolver.items().first())
    else {
        // No settings found, don't exclude
        return false;
    };

    excluded_by_settings(file_path, settings_item.value(), settings_item.path())
}

/// Whether `path` is filtered out by the `exclude`, `default-exclude` and
/// `include` patterns of `settings`, whose configuration directory is `root`.
///
/// The matchers come from `jarl_core::discovery`, which is what makes the LSP
/// and `jarl check` exclude the same files: one set of glob semantics, provided
/// by the `ignore` crate.
fn excluded_by_settings(path: &Path, settings: &Settings, root: &Path) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);

    let mut exclude: Vec<&str> = Vec::new();
    if let Some(patterns) = &settings.linter.exclude {
        exclude.extend(patterns.iter().map(String::as_str));
    }
    if settings.linter.default_exclude.unwrap_or(true) {
        exclude.extend_from_slice(DEFAULT_EXCLUDE_PATTERNS);
    }
    if let Some(overrides) = exclude_matcher(root, &exclude)
        && excluded_with_parents(&overrides, relative)
    {
        return true;
    }

    // If `include` is set, only files matching at least one pattern are linted.
    if let Some(patterns) = &settings.linter.include
        && let Some(overrides) = include_matcher(root, patterns)
        && !matches!(
            overrides.matched(relative, false),
            ignore::Match::Whitelist(_)
        )
    {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    use jarl_core::discovery::discover_r_file_paths;
    use jarl_core::settings::LinterSettings;

    /// R files laid out in the temporary project used by the tests below.
    const FILES: &[&str] = &[
        "file.R",
        "R/b.R",
        "R/generated/a.R",
        "R/generated/deep/c.R",
        "folder/d.R",
        "folder/sub/e.R",
        "renv/activate.R",
        "src/cpp11.R",
        "R/import-standalone-purrr.R",
    ];

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for file in FILES {
            let path = dir.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "x <- 1\n").unwrap();
        }
        dir
    }

    fn resolver_for(root: &Path, settings: Settings) -> PathResolver<Settings> {
        let mut resolver = PathResolver::new(Settings::default());
        resolver.add(root, settings);
        resolver
    }

    fn settings_with(exclude: Option<Vec<&str>>, include: Option<Vec<&str>>) -> Settings {
        Settings {
            linter: LinterSettings {
                exclude: exclude.map(|p| p.iter().map(|s| s.to_string()).collect()),
                include: include.map(|p| p.iter().map(|s| s.to_string()).collect()),
                ..Default::default()
            },
        }
    }

    /// The set of files `jarl check <root>` would lint.
    fn discovered(root: &Path, resolver: &PathResolver<Settings>) -> Vec<PathBuf> {
        discover_r_file_paths(&[root], &[], resolver, true, false)
            .into_iter()
            .filter_map(Result::ok)
            .collect()
    }

    /// The LSP and the CLI must exclude exactly the same files: both go through
    /// `jarl_core::discovery`, so a divergence here means one of the two grew a
    /// second implementation again.
    fn assert_lsp_agrees_with_cli(exclude: Option<Vec<&str>>, include: Option<Vec<&str>>) {
        let dir = project();
        let root = air_fs::normalize_path(dir.path());
        let resolver = resolver_for(&root, settings_with(exclude.clone(), include.clone()));
        let linted = discovered(&root, &resolver);

        for file in FILES {
            let path = air_fs::normalize_path(root.join(file));
            let cli_excludes = !linted.contains(&path);
            let lsp_excludes = should_exclude_file_based_on_settings(&path, &resolver);
            assert_eq!(
                lsp_excludes, cli_excludes,
                "disagreement on {file} (exclude = {exclude:?}, include = {include:?})"
            );
        }
    }

    #[test]
    fn agrees_on_default_excludes() {
        assert_lsp_agrees_with_cli(None, None);
    }

    #[test]
    fn agrees_on_recursive_glob() {
        assert_lsp_agrees_with_cli(Some(vec!["R/generated/**/*.R"]), None);
    }

    #[test]
    fn agrees_on_anchored_pattern() {
        assert_lsp_agrees_with_cli(Some(vec!["/file.R"]), None);
    }

    #[test]
    fn agrees_on_single_level_glob() {
        assert_lsp_agrees_with_cli(Some(vec!["folder/*.R"]), None);
    }

    #[test]
    fn agrees_on_directory_pattern() {
        assert_lsp_agrees_with_cli(Some(vec!["folder/"]), None);
    }

    #[test]
    fn agrees_on_negation() {
        assert_lsp_agrees_with_cli(Some(vec!["R/**", "!R/b.R"]), None);
    }

    #[test]
    fn agrees_on_include_patterns() {
        assert_lsp_agrees_with_cli(None, Some(vec!["R/**/*.R"]));
    }

    #[test]
    fn agrees_on_include_and_exclude() {
        assert_lsp_agrees_with_cli(Some(vec!["R/generated/**"]), Some(vec!["R/**/*.R"]));
    }

    #[test]
    fn recursive_glob_is_excluded() {
        // The regression this test exists for: the LSP used to glob the
        // basename only, so `R/generated/**/*.R` never matched anything.
        let dir = project();
        let root = air_fs::normalize_path(dir.path());
        let resolver = resolver_for(&root, settings_with(Some(vec!["R/generated/**/*.R"]), None));

        assert!(should_exclude_file_based_on_settings(
            &root.join("R/generated/a.R"),
            &resolver
        ));
        assert!(should_exclude_file_based_on_settings(
            &root.join("R/generated/deep/c.R"),
            &resolver
        ));
        assert!(!should_exclude_file_based_on_settings(
            &root.join("R/b.R"),
            &resolver
        ));
    }

    #[test]
    fn no_settings_excludes_nothing() {
        let resolver = PathResolver::new(Settings::default());
        assert!(!should_exclude_file_based_on_settings(
            &PathBuf::from("/project/renv/activate.R"),
            &resolver
        ));
    }
}
