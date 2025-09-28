use crate::{check, config::CheckConfig, discovery::discover_settings};
use air_workspace::resolve::PathResolver;
use std::fs;
use tempfile::Builder;

/// Test utility function to check if a given R code contains a specific lint
pub fn has_lint(text: &str, msg: &str, rule: &str, min_r_version: Option<&str>) -> bool {
    let temp_file = Builder::new()
        .prefix("test-flir")
        .suffix(".R")
        .tempfile()
        .unwrap();

    fs::write(&temp_file, text).expect("Failed to write initial content");

    let check_config = CheckConfig {
        files: vec![temp_file.path().to_path_buf()],
        fix: false,
        unsafe_fixes: false,
        fix_only: false,
        select_rules: rule.to_string(),
        ignore_rules: String::new(),
        min_r_version: min_r_version.map(|s| s.to_string()),
    };

    let mut resolver = PathResolver::new(crate::Settings::default());

    // Add discovered settings if any
    if let Ok(discovered) = discover_settings(&[temp_file.path().to_string_lossy().to_string()]) {
        for discovery in discovered {
            resolver.add(&discovery.directory, discovery.settings);
        }
    }

    let config = crate::config::build_config(
        &check_config,
        &resolver,
        vec![temp_file.path().to_path_buf()],
    )
    .expect("Failed to build config");

    let results = check(config);

    for (_, result) in results {
        if let Ok(diagnostics) = result {
            for diagnostic in diagnostics {
                if diagnostic.message.body.contains(msg) {
                    return true;
                }
            }
        }
    }

    false
}

/// Test utility function to check if a given R code does NOT contain a specific lint
pub fn has_no_lint(text: &str, rule: &str, min_r_version: Option<&str>) -> bool {
    let temp_file = Builder::new()
        .prefix("test-flir")
        .suffix(".R")
        .tempfile()
        .unwrap();

    fs::write(&temp_file, text).expect("Failed to write initial content");

    let check_config = CheckConfig {
        files: vec![temp_file.path().to_path_buf()],
        fix: false,
        unsafe_fixes: false,
        fix_only: false,
        select_rules: rule.to_string(),
        ignore_rules: String::new(),
        min_r_version: min_r_version.map(|s| s.to_string()),
    };

    let mut resolver = PathResolver::new(crate::Settings::default());

    // Add discovered settings if any
    if let Ok(discovered) = discover_settings(&[temp_file.path().to_string_lossy().to_string()]) {
        for discovery in discovered {
            resolver.add(&discovery.directory, discovery.settings);
        }
    }

    let config = crate::config::build_config(
        &check_config,
        &resolver,
        vec![temp_file.path().to_path_buf()],
    )
    .expect("Failed to build config");

    let results = check(config);

    for (_, result) in results {
        if let Ok(diagnostics) = result {
            if !diagnostics.is_empty() {
                return false;
            }
        }
    }

    true
}

/// Test utility to apply fixes to R code and return the fixed version
pub fn apply_fixes(
    text: &str,
    rule: &str,
    unsafe_fixes: bool,
    min_r_version: Option<&str>,
) -> String {
    let temp_file = Builder::new()
        .prefix("test-flir")
        .suffix(".R")
        .tempfile()
        .unwrap();

    fs::write(&temp_file, text).expect("Failed to write initial content");

    let check_config = CheckConfig {
        files: vec![temp_file.path().to_path_buf()],
        fix: true,
        unsafe_fixes,
        fix_only: false,
        select_rules: rule.to_string(),
        ignore_rules: String::new(),
        min_r_version: min_r_version.map(|s| s.to_string()),
    };

    let mut resolver = PathResolver::new(crate::Settings::default());

    // Add discovered settings if any
    if let Ok(discovered) = discover_settings(&[temp_file.path().to_string_lossy().to_string()]) {
        for discovery in discovered {
            resolver.add(&discovery.directory, discovery.settings);
        }
    }

    let config = crate::config::build_config(
        &check_config,
        &resolver,
        vec![temp_file.path().to_path_buf()],
    )
    .expect("Failed to build config");

    let _results = check(config);

    // Read the fixed content back
    fs::read_to_string(&temp_file).expect("Failed to read fixed content")
}

/// Check if code has any diagnostics for the given rule
pub fn check_code(text: &str, rule: &str, min_r_version: Option<&str>) -> Vec<crate::Diagnostic> {
    let temp_file = Builder::new()
        .prefix("test-flir")
        .suffix(".R")
        .tempfile()
        .unwrap();

    fs::write(&temp_file, text).expect("Failed to write initial content");

    let check_config = CheckConfig {
        files: vec![temp_file.path().to_path_buf()],
        fix: false,
        unsafe_fixes: false,
        fix_only: false,
        select_rules: rule.to_string(),
        ignore_rules: String::new(),
        min_r_version: min_r_version.map(|s| s.to_string()),
    };

    let mut resolver = PathResolver::new(crate::Settings::default());

    // Add discovered settings if any
    if let Ok(discovered) = discover_settings(&[temp_file.path().to_string_lossy().to_string()]) {
        for discovery in discovered {
            resolver.add(&discovery.directory, discovery.settings);
        }
    }

    let config = crate::config::build_config(
        &check_config,
        &resolver,
        vec![temp_file.path().to_path_buf()],
    )
    .expect("Failed to build config");

    let results = check(config);

    for (_, result) in results {
        if let Ok(diagnostics) = result {
            return diagnostics;
        }
    }

    Vec::new()
}

/// Convenience function to assert that code has no lint
pub fn expect_no_lint(text: &str, rule: &str, min_r_version: Option<&str>) {
    assert!(has_no_lint(text, rule, min_r_version));
}

/// Convenience function to assert that code has a specific lint
pub fn expect_lint(text: &str, msg: &str, rule: &str, min_r_version: Option<&str>) {
    assert!(has_lint(text, msg, rule, min_r_version));
}

/// Convenience function for no_lint (alias for has_no_lint)
pub fn no_lint(text: &str, rule: &str, min_r_version: Option<&str>) -> bool {
    has_no_lint(text, rule, min_r_version)
}

/// Get fixed text for a series of code snippets
pub fn get_fixed_text(text: Vec<&str>, rule: &str, min_r_version: Option<&str>) -> String {
    let mut output: String = String::new();

    for txt in text.iter() {
        let original_content = txt;
        let modified_content = apply_fixes(txt, rule, false, min_r_version);

        output.push_str(
            format!("  OLD:\n  ====\n{original_content}\n  NEW:\n  ====\n{modified_content}\n\n")
                .as_str(),
        );
    }

    output.trim_end().to_string()
}

/// Get fixed text with unsafe fixes for a series of code snippets
pub fn get_unsafe_fixed_text(text: Vec<&str>, rule: &str) -> String {
    let mut output: String = String::new();

    for txt in text.iter() {
        let original_content = txt;
        let modified_content = apply_fixes(txt, rule, true, None);

        output.push_str(
            format!("  OLD:\n  ====\n{original_content}\n  NEW:\n  ====\n{modified_content}\n\n")
                .as_str(),
        );
    }

    output.trim_end().to_string()
}
