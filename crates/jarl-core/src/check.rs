use crate::error::ParseError;
use crate::rule_set::Rule;
use crate::suppression::SuppressionManager;
use crate::vcs::check_version_control;
use air_fs::relativize_path;
use air_r_parser::RParserOptions;
use anyhow::{Context, Result};
use biome_rowan::TextSize;
use rayon::prelude::*;
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
// Re-exported so the LSP can pre-compute package duplicates before calling `check()`.
pub use crate::lints::base::duplicated_function_definition::duplicated_function_definition::compute_package_duplicate_assignments;
pub use crate::lints::base::duplicated_function_definition::duplicated_function_definition::is_in_r_package;
use crate::utils::*;

pub fn check(mut config: Config) -> Vec<(String, Result<Vec<Diagnostic>, anyhow::Error>)> {
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

    // Pre-compute cross-file duplicate assignments for the package rule.
    // This must happen before the config is wrapped in Arc.
    // Skip if already pre-populated (e.g. by the LSP which needs special
    // handling because it lints a temp file outside the real package tree).
    if config
        .rules_to_apply
        .contains(&Rule::DuplicatedFunctionDefinition)
        && config.package_duplicate_assignments.is_empty()
    {
        config.package_duplicate_assignments = compute_package_duplicate_assignments(&config.paths);
    }

    // Wrap config in Arc to avoid expensive clones in parallel execution
    let config = Arc::new(config);

    config
        .paths
        .par_iter()
        .map(|file| {
            let res = check_path(file, Arc::clone(&config));
            (relativize_path(file), res)
        })
        .collect()
}

pub fn check_path(path: &PathBuf, config: Arc<Config>) -> Result<Vec<Diagnostic>, anyhow::Error> {
    if config.apply_fixes || config.apply_unsafe_fixes {
        lint_fix(path, config)
    } else {
        lint_only(path, config)
    }
}

pub fn lint_only(path: &PathBuf, config: Arc<Config>) -> Result<Vec<Diagnostic>, anyhow::Error> {
    let path = relativize_path(path);
    let contents = fs::read_to_string(Path::new(&path))
        .with_context(|| format!("Failed to read file: {path}"))?;

    let checks = get_checks(&contents, &PathBuf::from(&path), &config)
        .with_context(|| format!("Failed to get checks for file: {path}"))?;

    Ok(checks)
}

pub fn lint_fix(path: &PathBuf, config: Arc<Config>) -> Result<Vec<Diagnostic>, anyhow::Error> {
    // Rmd/Qmd files never get autofixes applied.
    if crate::fs::has_rmd_extension(path) {
        return lint_only(path, config);
    }

    let path = relativize_path(path);

    let mut has_skipped_fixes = true;
    let mut checks: Vec<Diagnostic>;

    loop {
        let contents = fs::read_to_string(Path::new(&path))
            .with_context(|| format!("Failed to read file: {path}",))?;

        checks = get_checks(&contents, &PathBuf::from(&path), &config)
            .with_context(|| format!("Failed to get checks for file: {path}",))?;

        if !has_skipped_fixes {
            break;
        }

        let (new_has_skipped_fixes, fixed_text) = apply_fixes(&checks, &contents);
        has_skipped_fixes = new_has_skipped_fixes;

        fs::write(&path, fixed_text).with_context(|| format!("Failed to write file: {path}",))?;
    }

    Ok(checks)
}

// Takes the R code as a string, parses it, and obtains a (possibly empty)
// vector of `Diagnostic`s.
//
// If there are diagnostics to report, this is also where their range in the
// string is converted to their location (row, column).
pub fn get_checks(contents: &str, file: &Path, config: &Config) -> Result<Vec<Diagnostic>> {
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
    checker.package_duplicate_assignments = config
        .package_duplicate_assignments
        .get(file)
        .cloned()
        .unwrap_or_default();

    // We run checks at expression-level. This gathers all violations, no matter
    // whether they are suppressed or not. They are filtered out in the next
    // step (this is also Ruff's approach).
    for expr in expressions {
        check_expression(&expr, &mut checker)?;
    }

    // We run checks at document-level. This includes checks that require the
    // entire document (like top-level unreachable code) and comment-related
    // checks (blanket, unexplained, misplaced, misnamed, unused suppressions).
    // This must run after checking expressions because we filter out those that
    // are unused.
    check_document(expressions, &mut checker)?;

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

/// Lint an Rmd/Qmd file by extracting R code chunks and checking each one.
///
/// Key differences from regular R file linting:
/// - No autofix (Quarto code annotations make position-based edits unsafe)
/// - Diagnostic ranges are remapped from chunk-local byte offsets to file offsets
/// - `#| jarl-ignore-chunk` silently skips an entire chunk
/// - `#| jarl-ignore-file` suppression is applied across all chunks
fn get_checks_rmd(contents: &str, file: &Path, config: &Config) -> Result<Vec<Diagnostic>> {
    use std::collections::HashSet;

    let chunks = crate::rmd::extract_r_chunks(contents);

    struct ChunkState {
        parsed: air_r_parser::Parse,
        suppression: SuppressionManager,
        start_byte: usize,
    }

    // ── Pass 1: parse each chunk, build suppression managers,
    //    and collect file-level suppressed rules across all chunks ──
    let mut file_suppressed: HashSet<Rule> = HashSet::new();
    // Maps each file-level suppression comment to its rule, using file-level byte
    // offsets (chunk-local offset + chunk start_byte). Used later to remove
    // spurious outdated_suppression diagnostics for cross-chunk suppressions.
    let mut file_suppression_ranges: Vec<(biome_rowan::TextRange, Rule)> = Vec::new();
    let mut states: Vec<Option<ChunkState>> = Vec::with_capacity(chunks.len());

    for (chunk_index, chunk) in chunks.iter().enumerate() {
        let parsed = air_r_parser::parse(&chunk.code, RParserOptions::default());
        if parsed.has_error() {
            // Silently skip chunks with parse errors (e.g. documentation examples).
            states.push(None);
            continue;
        }
        let mut suppression = SuppressionManager::from_node(&parsed.syntax(), &chunk.code);

        // `# jarl-ignore-file` is only valid in the first R chunk (before any code).
        // In subsequent chunks it behaves like any other misplaced file suppression.
        if chunk_index > 0 {
            for fs in suppression.file_suppressions.drain(..) {
                suppression
                    .misplaced_file_suppressions
                    .push(fs.comment_range);
            }
        }

        let offset = TextSize::from(chunk.start_byte as u32);
        for fs in &suppression.file_suppressions {
            file_suppressed.insert(fs.rule);
            file_suppression_ranges.push((
                biome_rowan::TextRange::new(
                    fs.comment_range.start() + offset,
                    fs.comment_range.end() + offset,
                ),
                fs.rule,
            ));
        }
        states.push(Some(ChunkState {
            parsed,
            suppression,
            start_byte: chunk.start_byte,
        }));
    }

    // ── Pass 2: run lints on each chunk using its pre-built suppression manager ──
    let mut all_diagnostics: Vec<Diagnostic> = Vec::new();

    for state in states {
        let Some(ChunkState { parsed, suppression, start_byte }) = state else {
            continue;
        };

        let expressions = &parsed.tree().expressions();
        let mut checker = Checker::new(suppression, config.rule_options.clone());
        checker.rule_set = config.rules_to_apply.clone();
        checker.minimum_r_version = config.minimum_r_version;

        for expr in expressions {
            check_expression(&expr, &mut checker)?;
        }
        // check_document runs suppression filtering internally, so
        // checker.diagnostics is the post-suppression list after this call.
        check_document(expressions, &mut checker)?;

        let offset = TextSize::from(start_byte as u32);
        let diagnostics = checker.diagnostics.into_iter().map(|mut d| {
            d.filename = file.to_path_buf();
            d.fix = Fix::empty(); // no autofix for Rmd/Qmd
            // Remap range from chunk-local byte offsets to original file offsets.
            d.range = biome_rowan::TextRange::new(d.range.start() + offset, d.range.end() + offset);
            d
        });
        all_diagnostics.extend(diagnostics);
    }

    // A `# jarl-ignore-file` comment in one chunk can suppress violations in
    // other chunks. From the perspective of the chunk that contains the comment,
    // there are no local violations to suppress, so `check_document` marks the
    // suppression as unused and emits an `outdated_suppression` diagnostic.
    // Before the cross-chunk filter below removes the actual violations, we
    // identify which file-suppression comments are genuinely used cross-chunk
    // and remove the spurious outdated_suppression diagnostics for them.
    if !file_suppression_ranges.is_empty() {
        // Rules that have at least one real violation somewhere in the document.
        let rules_violated: HashSet<Rule> = all_diagnostics
            .iter()
            .filter(|d| d.message.name != "outdated_suppression")
            .filter_map(|d| Rule::from_name(&d.message.name))
            .filter(|r| file_suppressed.contains(r))
            .collect();

        if !rules_violated.is_empty() {
            // File-level suppression comment ranges that are actively used cross-chunk.
            let used_file_ranges: HashSet<biome_rowan::TextRange> = file_suppression_ranges
                .iter()
                .filter(|(_, rule)| rules_violated.contains(rule))
                .map(|(range, _)| *range)
                .collect();

            all_diagnostics.retain(|d| {
                !(d.message.name == "outdated_suppression" && used_file_ranges.contains(&d.range))
            });
        }
    }

    // Apply cross-chunk jarl-ignore-file suppressions.
    all_diagnostics.retain(|d| {
        Rule::from_name(&d.message.name)
            .map(|r| !file_suppressed.contains(&r))
            .unwrap_or(true)
    });

    let loc_new_lines = crate::utils::find_new_lines_from_content(contents);
    Ok(compute_lints_location(all_diagnostics, &loc_new_lines))
}
