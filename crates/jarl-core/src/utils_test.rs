use crate::check::check;
use crate::diagnostic::Diagnostic;
use crate::package_cache::PackageCache;
use crate::settings::Settings;
use crate::{config::ArgsConfig, discovery::discover_settings};
use air_workspace::resolve::PathResolver;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tempfile::Builder;

/// Declare a fake package namespace for use in tests.
///
/// Creates a `lazy_static` `NS` variable containing an `Arc<PackageCache>` built
/// from in-memory declarations. This avoids requiring R or installing packages
/// to run tests, which is important in CI.
///
/// # Example
///
/// ```ignore
/// declare_ns! {
///     "stats" => ["filter", "lag"],
///     "dplyr" => ["filter", "mutate", "select"],
/// }
///
/// fn snapshot_lint(code: &str) -> String {
///     format_diagnostics_with_cache(code, "my_rule", None, Some(&NS))
/// }
/// ```
#[macro_export]
macro_rules! declare_ns {
    ( $( $pkg:expr => [ $( $export:expr ),* $(,)? ] ),* $(,)? ) => {
        static NS: std::sync::LazyLock<std::sync::Arc<$crate::package_cache::PackageCache>> =
            std::sync::LazyLock::new(|| {
                std::sync::Arc::new($crate::package_cache::PackageCache::from_exports(&[
                    $( ($pkg, &[ $( $export ),* ]) ),*
                ]))
            });
    };
}

/// Set up the resolver, optionally with custom settings.
fn setup_resolver(file_path: &Path, settings: Option<Settings>) -> PathResolver<Settings> {
    let mut resolver = PathResolver::new(Settings::default());
    match settings {
        Some(s) => {
            resolver.add(file_path.parent().unwrap(), s);
        }
        None => {
            if let Ok(discovered) = discover_settings(&[file_path.to_string_lossy().to_string()]) {
                for discovery in discovered {
                    resolver.add(&discovery.directory, discovery.settings);
                }
            }
        }
    }
    resolver
}

/// Core helper: build a check config, optionally inject a package cache, and run
/// the linter. Returns diagnostics for the file.
fn run_check(
    text: &str,
    rule: &str,
    min_r_version: Option<&str>,
    settings: Option<Settings>,
    cache: Option<&Arc<PackageCache>>,
) -> Vec<Diagnostic> {
    let temp_file = Builder::new()
        .prefix("test-jarl")
        .suffix(".R")
        .tempfile()
        .unwrap();

    fs::write(&temp_file, text).expect("Failed to write initial content");

    let check_config = ArgsConfig {
        files: vec![temp_file.path().to_path_buf()],
        fix: false,
        unsafe_fixes: false,
        fix_only: false,
        select: rule.to_string(),
        extend_select: String::new(),
        ignore: String::new(),
        min_r_version: min_r_version.map(|s| s.to_string()),
        allow_dirty: false,
        allow_no_vcs: true,
        assignment: None,
    };

    let resolver = setup_resolver(temp_file.path(), settings);
    let toml_settings = resolver.items().first().map(|item| item.value());

    let mut config = crate::config::build_config(
        &check_config,
        toml_settings,
        vec![temp_file.path().to_path_buf()],
    )
    .expect("Failed to build config");

    if let Some(c) = cache {
        config.package_cache = Some(c.clone());
    }

    let results = check(config);

    for (_, result) in results {
        if let Ok(diagnostics) = result {
            return diagnostics;
        }
    }

    Vec::new()
}

/// Test utility to apply fixes to R code and return the fixed version
fn apply_fixes(
    text: &str,
    rule: &str,
    unsafe_fixes: bool,
    min_r_version: Option<&str>,
    settings: Option<Settings>,
) -> String {
    let temp_file = Builder::new()
        .prefix("test-jarl")
        .suffix(".R")
        .tempfile()
        .unwrap();

    fs::write(&temp_file, text).expect("Failed to write initial content");

    let check_config = ArgsConfig {
        files: vec![temp_file.path().to_path_buf()],
        fix: true,
        unsafe_fixes,
        fix_only: false,
        select: rule.to_string(),
        extend_select: String::new(),
        ignore: String::new(),
        min_r_version: min_r_version.map(|s| s.to_string()),
        allow_dirty: false,
        allow_no_vcs: true,
        assignment: None,
    };

    let resolver = setup_resolver(temp_file.path(), settings);
    let toml_settings = resolver.items().first().map(|item| item.value());

    let config = crate::config::build_config(
        &check_config,
        toml_settings,
        vec![temp_file.path().to_path_buf()],
    )
    .expect("Failed to build config");

    let _results = check(config);

    // Read the fixed content back
    fs::read_to_string(&temp_file).expect("Failed to read fixed content")
}

/// Check if code has any diagnostics for the given rule
pub fn check_code(text: &str, rule: &str, min_r_version: Option<&str>) -> Vec<Diagnostic> {
    run_check(text, rule, min_r_version, None, None)
}

/// Convenience function to assert that code has no lint
pub fn expect_no_lint(text: &str, rule: &str, min_r_version: Option<&str>) {
    let diagnostics = run_check(text, rule, min_r_version, None, None);
    assert!(
        diagnostics.is_empty(),
        "Expected no lint for rule '{rule}' but got {} diagnostic(s)",
        diagnostics.len()
    );
}

/// Convenience function to assert that code has no lint, with custom settings
pub fn expect_no_lint_with_settings(
    text: &str,
    rule: &str,
    min_r_version: Option<&str>,
    settings: Settings,
) {
    let diagnostics = run_check(text, rule, min_r_version, Some(settings), None);
    assert!(
        diagnostics.is_empty(),
        "Expected no lint for rule '{rule}' but got {} diagnostic(s)",
        diagnostics.len()
    );
}

/// Get fixed text for a series of code snippets
pub fn get_fixed_text(text: Vec<&str>, rule: &str, min_r_version: Option<&str>) -> String {
    get_fixed_text_with_settings(text, rule, min_r_version, None)
}

/// Get fixed text for a series of code snippets, with custom settings
pub fn get_fixed_text_with_settings(
    text: Vec<&str>,
    rule: &str,
    min_r_version: Option<&str>,
    settings: Option<Settings>,
) -> String {
    let mut output: String = String::new();

    for txt in text.iter() {
        let original_content = txt;
        let modified_content = apply_fixes(txt, rule, false, min_r_version, settings.clone());

        output.push_str(
            format!("OLD:\n====\n{original_content}\nNEW:\n====\n{modified_content}\n\n").as_str(),
        );
    }

    output.trim_end().to_string()
}

/// Extract the highlighted text based on the diagnostic range for a given rule
///
/// This function runs the linter on the provided code and returns the exact text
/// that would be highlighted in the LSP, based on the diagnostic range. This is
/// needed when the range reported by the diagnostic is different from the range
/// reported in the fix, e.g. for `assignment` linter.
///
/// # Arguments
/// - `text` - The R code to analyze
/// - `rule` - The rule name to check
/// - `expected_highlight` - The expected text that should be highlighted
///
/// # Example
/// ```
/// expect_diagnostic_highlight("x = 1", "assignment", "x =");
/// expect_diagnostic_highlight("1 -> x", "assignment", "-> x");
/// ```
pub fn expect_diagnostic_highlight(text: &str, rule: &str, expected_highlight: &str) {
    let highlighted = get_diagnostic_highlight(text, rule, None);
    assert_eq!(
        highlighted, expected_highlight,
        "Expected highlight '{expected_highlight}' but got '{highlighted}' for rule '{rule}' on code: {text}"
    );
}

/// Get the highlighted text based on the diagnostic range for a given rule
///
/// Returns the exact text that would be highlighted in the LSP.
pub fn get_diagnostic_highlight(text: &str, rule: &str, min_r_version: Option<&str>) -> String {
    let diagnostics = check_code(text, rule, min_r_version);

    if diagnostics.is_empty() {
        panic!("No diagnostics found for rule '{rule}' on code: {text}");
    }

    if diagnostics.len() > 1 {
        panic!(
            "Multiple diagnostics found for rule '{rule}' on code: {text}. Expected exactly one."
        );
    }

    let diagnostic = &diagnostics[0];
    let range = diagnostic.range;

    // Extract the text within the diagnostic range
    let start_offset = usize::from(range.start());
    let end_offset = usize::from(range.end());

    if end_offset > text.len() || start_offset > end_offset {
        panic!(
            "Invalid range [{}, {}) for text of length {} on code: {}",
            start_offset,
            end_offset,
            text.len(),
            text
        );
    }

    text[start_offset..end_offset].to_string()
}

/// Get fixed text with unsafe fixes for a series of code snippets
pub fn get_unsafe_fixed_text(text: Vec<&str>, rule: &str) -> String {
    get_unsafe_fixed_text_with_settings(text, rule, None)
}

/// Get fixed text with unsafe fixes for a series of code snippets, with custom settings
pub fn get_unsafe_fixed_text_with_settings(
    text: Vec<&str>,
    rule: &str,
    settings: Option<Settings>,
) -> String {
    let mut output: String = String::new();

    for txt in text.iter() {
        let original_content = txt;
        let modified_content = apply_fixes(txt, rule, true, None, settings.clone());

        output.push_str(
            format!("OLD:\n====\n{original_content}\nNEW:\n====\n{modified_content}\n\n").as_str(),
        );
    }

    output.trim_end().to_string()
}

/// Format diagnostics as they would appear in the console for snapshot testing.
pub fn format_diagnostics(text: &str, rule: &str, min_r_version: Option<&str>) -> String {
    render_diagnostics(text, rule, min_r_version, None, None)
}

/// Format diagnostics with custom settings for snapshot testing.
pub fn format_diagnostics_with_settings(
    text: &str,
    rule: &str,
    min_r_version: Option<&str>,
    settings: Option<Settings>,
) -> String {
    render_diagnostics(text, rule, min_r_version, settings, None)
}

/// Format diagnostics with a fake package cache for snapshot testing.
pub fn format_diagnostics_with_cache(
    text: &str,
    rule: &str,
    min_r_version: Option<&str>,
    cache: &Arc<PackageCache>,
) -> String {
    render_diagnostics(text, rule, min_r_version, None, Some(cache))
}

fn render_diagnostics(
    text: &str,
    rule: &str,
    min_r_version: Option<&str>,
    settings: Option<Settings>,
    cache: Option<&Arc<PackageCache>>,
) -> String {
    use annotate_snippets::Renderer;

    use crate::diagnostic::render_diagnostic;

    let diagnostics = run_check(text, rule, min_r_version, settings, cache);

    if diagnostics.is_empty() {
        return "All checks passed!".to_string();
    }

    // Force plain rendering for consistent snapshots (no colors)
    let renderer = Renderer::plain();

    let mut output = String::new();

    for diagnostic in &diagnostics {
        let rendered = render_diagnostic(
            text,
            "<test>",
            &diagnostic.message.name,
            diagnostic,
            &renderer,
        );
        output.push_str(&format!("{}\n", rendered));
    }

    output.push_str(&format!(
        "Found {} error{}.",
        diagnostics.len(),
        if diagnostics.len() == 1 { "" } else { "s" }
    ));

    output
}
