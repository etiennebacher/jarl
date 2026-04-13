use crate::error::ParseError;
use crate::package::{
    FilePackageInfo, FileScope, PackageAnalysis, PackageContext, make_package_analysis,
    summarize_package_info,
};
use crate::roxygen::{extract_roxygen_examples, remap_roxygen_fix, remap_roxygen_range};
use crate::suppression::SuppressionManager;
use crate::vcs::check_version_control;
use air_fs::relativize_path;
use air_r_parser::RParserOptions;
use air_r_syntax::{RExpressionList, RSyntaxNode};
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use crate::analyze::document::check_document;
use crate::analyze::expression::check_expression;
pub use crate::checker::Checker;
use crate::config::Config;
use crate::diagnostic::*;
use crate::fix::*;
use crate::utils::*;

pub fn check(config: Config) -> Vec<(String, Result<Vec<Diagnostic>, anyhow::Error>)> {
    let (pkg_contexts, file_pkg_info) = summarize_package_info(&config.paths);

    let namespace_contents: HashMap<PathBuf, String> = pkg_contexts
        .iter()
        .filter_map(|(root, ctx)| {
            ctx.namespace_content
                .as_ref()
                .map(|c| (root.clone(), c.clone()))
        })
        .collect();
    let pkg = make_package_analysis(&config.paths, &config, &namespace_contents);
    let pkg_contexts = Arc::new(pkg_contexts);
    let file_pkg_info = Arc::new(file_pkg_info);

    // Ensure that all paths are covered by VCS. This is conservative because
    // technically we could apply fixes on those that are covered by VCS and
    // error for the others, but I'd rather be on the safe side and force the
    // user to deal with that before applying any fixes.
    if (config.apply_fixes || config.apply_unsafe_fixes) && !config.paths.is_empty() {
        let path_strings: Vec<String> = config.paths.iter().map(relativize_path).collect();
        if let Err(e) = check_version_control(&path_strings, &config) {
            let first_path = path_strings.first().unwrap().clone();
            return vec![(first_path, Err(e))];
        }
    }

    // Wrap config and package analysis in Arc to avoid expensive clones in parallel execution
    let config = Arc::new(config);
    let pkg = Arc::new(pkg);

    config
        .paths
        .par_iter()
        .map(|file| {
            let res = check_path(
                file,
                Arc::clone(&config),
                Arc::clone(&pkg),
                Arc::clone(&pkg_contexts),
                Arc::clone(&file_pkg_info),
            );
            (relativize_path(file), res)
        })
        .collect()
}

pub fn check_path(
    path: &PathBuf,
    config: Arc<Config>,
    pkg: Arc<PackageAnalysis>,
    pkg_contexts: Arc<HashMap<PathBuf, PackageContext>>,
    file_pkg_info: Arc<HashMap<PathBuf, FilePackageInfo>>,
) -> Result<Vec<Diagnostic>, anyhow::Error> {
    if config.apply_fixes || config.apply_unsafe_fixes {
        lint_fix(path, config, pkg, pkg_contexts, file_pkg_info)
    } else {
        lint_only(path, config, pkg, pkg_contexts, file_pkg_info)
    }
}

pub fn lint_only(
    path: &PathBuf,
    config: Arc<Config>,
    pkg: Arc<PackageAnalysis>,
    pkg_contexts: Arc<HashMap<PathBuf, PackageContext>>,
    file_pkg_info: Arc<HashMap<PathBuf, FilePackageInfo>>,
) -> Result<Vec<Diagnostic>, anyhow::Error> {
    let path = relativize_path(path);
    let contents = fs::read_to_string(Path::new(&path))
        .with_context(|| format!("Failed to read file: {path}"))?;

    let checks = get_checks(
        &contents,
        &PathBuf::from(&path),
        &config,
        &pkg,
        &pkg_contexts,
        &file_pkg_info,
    )
    .with_context(|| format!("Failed to get checks for file: {path}"))?;

    Ok(checks)
}

pub fn lint_fix(
    path: &PathBuf,
    config: Arc<Config>,
    pkg: Arc<PackageAnalysis>,
    pkg_contexts: Arc<HashMap<PathBuf, PackageContext>>,
    file_pkg_info: Arc<HashMap<PathBuf, FilePackageInfo>>,
) -> Result<Vec<Diagnostic>, anyhow::Error> {
    // Rmd/Qmd files never get autofixes applied.
    if crate::fs::has_rmd_extension(path) {
        return lint_only(path, config, pkg, pkg_contexts, file_pkg_info);
    }

    let path = relativize_path(path);

    let mut checks: Vec<Diagnostic>;

    loop {
        let contents = fs::read_to_string(Path::new(&path))
            .with_context(|| format!("Failed to read file: {path}",))?;

        checks = get_checks(
            &contents,
            &PathBuf::from(&path),
            &config,
            &pkg,
            &pkg_contexts,
            &file_pkg_info,
        )
        .with_context(|| format!("Failed to get checks for file: {path}",))?;

        let has_fixable = checks
            .iter()
            .any(|d| d.has_safe_fix() || d.has_unsafe_fix());
        if !has_fixable {
            break;
        }

        let fixed_text = apply_fixes(&checks, &contents);

        // No progress was made (e.g. all fixes overlap), stop to avoid an
        // infinite loop.
        if fixed_text == contents {
            break;
        }

        fs::write(&path, fixed_text).with_context(|| format!("Failed to write file: {path}",))?;
    }

    Ok(checks)
}

// Takes the R code as a string, parses it, and obtains a (possibly empty)
// vector of `Diagnostic`s.
//
// If there are diagnostics to report, this is also where their range in the
// string is converted to their location (row, column).
pub fn get_checks(
    contents: &str,
    file: &Path,
    config: &Config,
    pkg: &PackageAnalysis,
    pkg_contexts: &HashMap<PathBuf, PackageContext>,
    file_pkg_info: &HashMap<PathBuf, FilePackageInfo>,
) -> Result<Vec<Diagnostic>> {
    if crate::fs::has_rmd_extension(file) {
        return get_checks_rmd(contents, file, config);
    }

    let parser_options = RParserOptions::default();
    let parsed = air_r_parser::parse(contents, parser_options);

    if parsed.has_error() {
        return Err(ParseError { filename: file.to_path_buf() }.into());
    }

    let syntax = &parsed.syntax();
    let expressions = &parsed.tree().expressions();

    let suppression = SuppressionManager::from_node(syntax, contents);

    let mut checker = Checker::new(suppression, config.rule_options.clone());
    checker.rule_set = config.rules_to_apply.clone();
    checker.minimum_r_version = config.minimum_r_version;

    // Wire up package context for package-specific rules.
    get_package_info(
        &mut checker,
        file,
        expressions,
        config,
        pkg_contexts,
        file_pkg_info,
    );

    // Look up per-file data from PackageAnalysis
    let duplicate_assignments = pkg
        .duplicate_assignments
        .get(file)
        .cloned()
        .unwrap_or_default();
    let unused_functions = pkg.unused_functions.get(file).cloned().unwrap_or_default();

    // We run checks at expression-level. This gathers all violations, no matter
    // whether they are suppressed or not. They are filtered out in the next
    // step (this is also Ruff's approach).
    for expr in expressions {
        check_expression(&expr, &mut checker)?;
    }

    // Lint R code inside roxygen @examples / @examplesIf sections.
    // Collected before check_document so that suppression filtering (which
    // runs inside check_document) can match `# jarl-ignore` comments in
    // the main file against violations found in roxygen examples.
    if config.check_roxygen
        && contents.contains("#'")
        && contents.contains("@examples")
        && matches!(
            file_pkg_info.get(file),
            Some(FilePackageInfo::InPackage { scope: FileScope::R, .. })
        )
    {
        let roxygen_diagnostics = get_checks_roxygen(syntax, file, config, contents)?;
        checker.diagnostics.extend(roxygen_diagnostics);
    }

    // We run checks at document-level. This includes checks that require the
    // entire document (like top-level unreachable code) and comment-related
    // checks (blanket, unexplained, misplaced, misnamed, unused suppressions).
    // This must run after checking expressions because we filter out those that
    // are unused.
    check_document(
        expressions,
        &mut checker,
        &duplicate_assignments,
        &unused_functions,
    )?;

    // Some rules have a fix available in their implementation but do not have
    // fix in the config, for instance because they are part of the "unfixable"
    // arg or not part of the "fixable" arg in `jarl.toml`.
    // When we get all the diagnostics with check_expression() above, we don't
    // pay attention to whether the user wants to fix them or not. Adding this
    // step here is a way to filter those fixes out before calling apply_fixes().
    let rules_without_fix = checker
        .rule_set
        .iter()
        .filter(|x| x.has_no_fix())
        .map(|x| x.name().to_string())
        .collect::<Vec<String>>();

    let diagnostics: Vec<Diagnostic> = checker
        .diagnostics
        .into_iter()
        .map(|mut x| {
            x.filename = file.to_path_buf();
            // Check if fix should be skipped based on fixable/unfixable settings
            if rules_without_fix.contains(&x.message.name) {
                x.fix = Fix::empty();
            }
            // Also check against unfixable set from config
            if config.unfixable.contains(&x.message.name) {
                x.fix = Fix::empty();
            }
            // If fixable is specified, only allow those rules to have fixes
            if let Some(ref fixable_set) = config.fixable
                && !fixable_set.contains(&x.message.name)
            {
                x.fix = Fix::empty();
            }
            // TODO: this should be removed once comments in nodes are better
            // handled, #95
            if x.fix.to_skip {
                x.fix = Fix::empty();
            }
            x
        })
        .collect();

    let loc_new_lines = find_new_lines(syntax)?;
    let diagnostics = compute_lints_location(diagnostics, &loc_new_lines);

    Ok(diagnostics)
}

/// Populate package context on the checker from pre-computed data.
///
/// For files inside an R package, copies the pre-computed `PackageContext`
/// fields. For scripts, scans for `library()`/`require()` calls.
fn get_package_info(
    checker: &mut Checker,
    file: &Path,
    expressions: &RExpressionList,
    config: &Config,
    pkg_contexts: &HashMap<PathBuf, PackageContext>,
    file_pkg_info: &HashMap<PathBuf, FilePackageInfo>,
) {
    match file_pkg_info.get(file) {
        Some(FilePackageInfo::InPackage { package_root, .. }) => {
            if let Some(ctx) = pkg_contexts.get(package_root) {
                checker.loaded_packages = ctx.loaded_packages.clone();
                checker.import_from = ctx.import_from.clone();
                checker.namespace_exports = ctx.namespace_exports.clone();
            }
        }
        _ => {
            let mut packages: Vec<String> = crate::checker::DEFAULT_PACKAGES
                .iter()
                .map(|s| s.to_string())
                .collect();
            packages.extend(crate::library_calls::extract_library_calls(expressions));
            checker.loaded_packages = packages;
        }
    }
    checker.package_cache = config.package_cache.clone();
}

/// Lint R code inside roxygen `@examples` and `@examplesIf` sections.
///
/// Each examples section is extracted, parsed as standalone R code, and linted.
/// Diagnostic byte ranges are remapped to point to the correct position in the
/// original file. Autofixes are disabled because the `#'` prefix makes
/// position-based edits unsafe.
fn get_checks_roxygen(
    syntax: &RSyntaxNode,
    file: &Path,
    config: &Config,
    contents: &str,
) -> Result<Vec<Diagnostic>> {
    let chunks = extract_roxygen_examples(syntax, contents);
    let mut all_diagnostics: Vec<Diagnostic> = Vec::new();

    for chunk in &chunks {
        let parsed = air_r_parser::parse(&chunk.code, RParserOptions::default());
        if parsed.has_error() {
            // Examples may contain pseudo-code, \dontrun{} wrappers, etc.
            continue;
        }

        let expressions = &parsed.tree().expressions();
        let suppression = SuppressionManager::from_node(&parsed.syntax(), &chunk.code);
        let has_suppressions = suppression.has_any_suppressions;
        let mut checker = Checker::new(suppression, config.rule_options.clone());
        checker.rule_set = config.rules_to_apply.clone();
        checker.minimum_r_version = config.minimum_r_version;

        for expr in expressions {
            check_expression(&expr, &mut checker)?;
        }

        // Only run document-level checks if the examples code has inline
        // suppression comments. Most examples don't, and check_document is
        // otherwise unnecessary here (no package-level analysis, no
        // suppression-related diagnostics to report).
        if has_suppressions {
            check_document(expressions, &mut checker, &[], &[])?;
        }

        for mut d in checker.diagnostics {
            d.range = remap_roxygen_range(d.range, chunk);
            if config.fix_roxygen {
                d.fix = remap_roxygen_fix(&d.fix, chunk, contents);
            } else {
                d.fix = Fix::empty();
            }
            d.filename = file.to_path_buf();
            all_diagnostics.push(d);
        }
    }

    Ok(all_diagnostics)
}

/// Lint an Rmd/Qmd file by concatenating R code chunks into a virtual R
/// string and running the normal linting pipeline on it.
///
/// Key differences from regular R file linting:
/// - No autofix (Quarto code annotations make position-based edits unsafe)
/// - `#| jarl-ignore-chunk:` YAML blocks are translated to `# jarl-ignore-start`
///   / `# jarl-ignore-end` pairs before linting
/// - Chunks with parse errors are silently dropped
/// - Diagnostic ranges are remapped from virtual-string offsets to original file offsets
fn get_checks_rmd(contents: &str, file: &Path, config: &Config) -> Result<Vec<Diagnostic>> {
    let chunks = crate::rmd::extract_r_chunks(contents);
    let (virtual_source, offset_map) = crate::rmd::build_virtual_r_source(&chunks);

    if virtual_source.trim().is_empty() {
        return Ok(Vec::new());
    }

    let parsed = air_r_parser::parse(&virtual_source, RParserOptions::default());
    if parsed.has_error() {
        return Err(crate::error::ParseError { filename: file.to_path_buf() }.into());
    }

    let suppression = SuppressionManager::from_node(&parsed.syntax(), &virtual_source);
    let mut checker = Checker::new(suppression, config.rule_options.clone());
    checker.rule_set = config.rules_to_apply.clone();
    checker.minimum_r_version = config.minimum_r_version;

    let expressions = &parsed.tree().expressions();
    for expr in expressions {
        check_expression(&expr, &mut checker)?;
    }
    // check_document runs suppression filtering internally, so
    // checker.diagnostics is the post-suppression list after this call.
    // Rmd chunks don't participate in package-level analysis, so pass empty slices.
    check_document(expressions, &mut checker, &[], &[])?;

    // Remap ranges from virtual-string offsets to original Rmd file offsets.
    let diagnostics: Vec<Diagnostic> = checker
        .diagnostics
        .into_iter()
        .map(|mut d| {
            d.filename = file.to_path_buf();
            d.fix = Fix::empty();
            d.range = offset_map.remap_range(d.range);
            d
        })
        .collect();

    let loc_new_lines = crate::utils::find_new_lines_from_content(contents);
    Ok(compute_lints_location(diagnostics, &loc_new_lines))
}

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    #[test]
    fn test_fix_does_not_introduce_new_lints() {
        // Fixing `outer_negation` on this code would produce
        // `expect_true(!any(is.na(x)))`, which introduced new
        // `expect_not` and `any_is_na` lints. The fix loop should keep
        // going until the code is fully clean.
        assert_snapshot!(
            get_fixed_text(
                vec!["expect_true(all(!is.na(x)))"],
                "ALL",
                None
            ),
            @"
        OLD:
        ====
        expect_true(all(!is.na(x)))
        NEW:
        ====
        expect_false(anyNA(x))
        "
        );
    }

    #[test]
    fn test_overlapping_fixes_do_not_corrupt() {
        // `fixed_regex` replaces the whole call (adding `, fixed = TRUE`)
        // while `quotes` replaces just the string inside it. The nested
        // fix must be skipped in the first pass and applied in the next
        // iteration, not applied on stale offsets.
        assert_snapshot!(
            get_fixed_text(
                vec!["grepl('/', repo)"],
                "ALL",
                None
            ),
            @r#"
        OLD:
        ====
        grepl('/', repo)
        NEW:
        ====
        grepl("/", repo, fixed = TRUE)
        "#
        );
    }
}
