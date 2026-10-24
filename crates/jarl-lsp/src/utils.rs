//! Utility functions for the Jarl LSP server

pub use jarl_core::discovery::should_exclude_file_based_on_settings;

#[cfg(test)]
mod tests {
    use super::*;
    use air_workspace::resolve::PathResolver;
    use jarl_core::settings::Settings;
    use std::fs;
    use std::path::Path;
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
        let mut resolver = PathResolver::new();
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
        let resolver = PathResolver::new();
        assert!(!should_exclude_file_based_on_settings(
            &PathBuf::from("/project/renv/activate.R"),
            &resolver
        ));
    }
}
