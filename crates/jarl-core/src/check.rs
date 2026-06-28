use crate::error::ParseError;
use crate::package::{
    FilePackageInfo, FileScope, PackageAnalysis, PackageContext, PackageFileAnalysis,
    make_package_analysis, make_package_analysis_deferred, summarize_package_info,
};
use crate::roxygen::{extract_roxygen_examples, remap_roxygen_fix, remap_roxygen_range};
use crate::suppression::{SuppressionFilter, SuppressionManager};
use crate::vcs::check_version_control;
use air_fs::relativize_path;
use air_r_parser::RParserOptions;
use air_r_syntax::{RExpressionList, RSyntaxNode};
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use crate::analyze::document::check_document;
use crate::analyze::expression::check_expression;
pub use crate::checker::Checker;
use crate::config::Config;
use crate::db::FileUses;
use crate::diagnostic::*;
use crate::fix::*;
use crate::lints::comments::outdated_suppression::outdated_suppression::outdated_suppression;
use crate::rule_set::{Rule, RuleSet};
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

    // Fix mode keeps the two-pass design (a cross-file pre-pass that parses
    // every package file, then a per-file fix loop) because the loop rewrites
    // files between iterations and re-parses each one anyway. Read-only linting
    // uses the fused single-parse pass, which is where the per-file parse would
    // otherwise be paid twice.
    if config.apply_fixes || config.apply_unsafe_fixes {
        return check_with_fixes(config, pkg_contexts, file_pkg_info, &namespace_contents);
    }

    check_lint_only_fused(config, pkg_contexts, file_pkg_info, &namespace_contents)
}

/// Apply fixes: validate VCS coverage, run the cross-file pre-pass, then fix
/// each file in parallel.
fn check_with_fixes(
    config: Config,
    pkg_contexts: HashMap<PathBuf, PackageContext>,
    file_pkg_info: HashMap<PathBuf, FilePackageInfo>,
    namespace_contents: &HashMap<PathBuf, String>,
) -> Vec<(String, Result<Vec<Diagnostic>, anyhow::Error>)> {
    // Ensure that all paths are covered by VCS. This is conservative because
    // technically we could apply fixes on those that are covered by VCS and
    // error for the others, but I'd rather be on the safe side and force the
    // user to deal with that before applying any fixes.
    if !config.paths.is_empty() {
        let path_strings: Vec<String> = config.paths.iter().map(relativize_path).collect();
        if let Err(e) = check_version_control(&path_strings, &config) {
            let first_path = path_strings.first().unwrap().clone();
            return vec![(first_path, Err(e))];
        }
    }

    let pkg = make_package_analysis(&config.paths, &config, namespace_contents);
    let pkg_contexts = Arc::new(pkg_contexts);
    let file_pkg_info = Arc::new(file_pkg_info);

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

/// One target file's phase-1 result: its cross-file contribution (if it belongs
/// to the package namespace) and either a finished diagnostic list or the
/// deferred state to finalize once cross-file usage is known.
struct TargetOutput {
    /// Relativized path, the key used by the cross-file map and the output.
    rel: PathBuf,
    /// `Some` only for package-namespace files when `unused_object` runs.
    file_uses: Option<FileUses>,
    lint: LintOutcome,
}

enum LintOutcome {
    /// Already complete: Rmd, generated, parse error, or a run with no
    /// cross-file `unused_object` to resolve.
    Final(Result<Vec<Diagnostic>, anyhow::Error>),
    /// Needs phase 2: drop cross-file-used top-level objects, then filter
    /// suppressions and compute locations. Boxed because it is much larger than
    /// the `Final` variant and most targets in a package take this path.
    Deferred(Box<DeferredLint>),
}

/// `Send` state carried from phase 1 to phase 2 for a target whose top-level
/// `unused_object` decision was deferred. Holds no syntax tree — only the
/// raw diagnostics, the provisional top-level diagnostics keyed by name, and a
/// snapshot of the suppression state.
struct DeferredLint {
    diagnostics: Vec<Diagnostic>,
    provisional: Vec<(String, Diagnostic)>,
    suppression: SuppressionFilter,
    new_lines: Vec<usize>,
    rule_set: RuleSet,
    outdated_enabled: bool,
}

/// Read-only linting that parses each file exactly once.
///
/// A package's R files share one namespace, so `unused_object` must know
/// whether a file's top-level object is read by a sibling before flagging it.
/// The two-pass design paid for that by parsing every package file in a
/// cross-file pre-pass and then parsing each linted file again. This fuses the
/// two: every file is parsed once, targets are linted with the cross-file
/// decision deferred, and a cheap merge resolves it afterwards.
fn check_lint_only_fused(
    config: Config,
    pkg_contexts: HashMap<PathBuf, PackageContext>,
    file_pkg_info: HashMap<PathBuf, FilePackageInfo>,
    namespace_contents: &HashMap<PathBuf, String>,
) -> Vec<(String, Result<Vec<Diagnostic>, anyhow::Error>)> {
    let (pkg, db) = make_package_analysis_deferred(&config.paths, &config, namespace_contents);

    let check_unused_object = config.rules_to_apply.contains(&Rule::UnusedObject);

    // The package-namespace universe (every scanned R file). A target's
    // top-level object counts as used when a sibling here reads it. Only files
    // in this set participate in cross-file resolution, so loose scripts never
    // pollute (or borrow from) a package's namespace.
    let universe: Vec<PathBuf> = if check_unused_object {
        db.as_ref()
            .map(|db| db.universe_paths())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let universe_set: HashSet<PathBuf> = universe.iter().cloned().collect();

    let config = Arc::new(config);
    let pkg = Arc::new(pkg);
    let pkg_contexts = Arc::new(pkg_contexts);
    let file_pkg_info = Arc::new(file_pkg_info);

    // Targets: the files we emit diagnostics for. Keyed by relativized path to
    // match the universe and cross-file maps.
    let targets: Vec<PathBuf> = config
        .paths
        .iter()
        .map(|p| PathBuf::from(relativize_path(p)))
        .collect();
    let target_set: HashSet<&PathBuf> = targets.iter().collect();

    // Sibling files: in the package namespace but not lint targets. Parsed once
    // only to contribute their free uses to cross-file resolution.
    let extra_universe: Vec<&PathBuf> = universe
        .iter()
        .filter(|p| !target_set.contains(*p))
        .collect();

    // PHASE 1: parse each file once. Targets get the full lint (with the
    // top-level `unused_object` decision deferred); siblings only contribute
    // their free uses.
    let target_outputs: Vec<TargetOutput> = targets
        .par_iter()
        .map(|rel| {
            let in_universe = universe_set.contains(rel);
            process_target(
                rel,
                &config,
                &pkg,
                &pkg_contexts,
                &file_pkg_info,
                check_unused_object && in_universe,
            )
        })
        .collect();

    let sibling_uses: Vec<FileUses> = if check_unused_object {
        extra_universe
            .par_iter()
            .filter_map(|path| extract_file_uses(path))
            .collect()
    } else {
        Vec::new()
    };

    // Split phase-1 results: cross-file contributions feed the merge, lint
    // outcomes go to phase 2.
    let mut target_uses: Vec<FileUses> = Vec::new();
    let mut lints: Vec<(PathBuf, LintOutcome)> = Vec::with_capacity(target_outputs.len());
    for output in target_outputs {
        if let Some(uses) = output.file_uses {
            target_uses.push(uses);
        }
        lints.push((output.rel, output.lint));
    }

    // MERGE: which top-level names are read across files.
    let cross_file_used = if check_unused_object {
        let mut all_uses = sibling_uses;
        all_uses.append(&mut target_uses);
        crate::db::merge_cross_file(&all_uses)
    } else {
        HashMap::new()
    };

    // PHASE 2: drop cross-file-used objects, then finalize each target.
    let mut by_path: HashMap<PathBuf, Result<Vec<Diagnostic>, anyhow::Error>> = HashMap::new();
    for (rel, lint) in lints {
        let result = finalize_target(lint, &rel, &config, &cross_file_used);
        by_path.insert(rel, result);
    }

    // Emit results in `config.paths` order.
    config
        .paths
        .iter()
        .map(|orig| {
            let rel = PathBuf::from(relativize_path(orig));
            let result = by_path.remove(&rel).unwrap_or_else(|| Ok(Vec::new()));
            (relativize_path(orig), result)
        })
        .collect()
}

/// Phase 1 for one lint target: parse once, then lint. `extract` is set when the
/// file belongs to the package namespace and `unused_object` runs, in which case
/// its top-level definitions and free uses are read off the same index the lint
/// uses.
fn process_target(
    rel: &Path,
    config: &Config,
    pkg: &PackageAnalysis,
    pkg_contexts: &HashMap<PathBuf, PackageContext>,
    file_pkg_info: &HashMap<PathBuf, FilePackageInfo>,
    extract: bool,
) -> TargetOutput {
    let output = |file_uses, lint| TargetOutput { rel: rel.to_path_buf(), file_uses, lint };

    // The error contexts mirror those the two-pass path added in `lint_only`.
    let contents = match fs::read_to_string(rel) {
        Ok(contents) => contents,
        Err(e) => {
            let err =
                anyhow::Error::new(e).context(format!("Failed to read file: {}", rel.display()));
            return output(None, LintOutcome::Final(Err(err)));
        }
    };

    // Rmd/Qmd: self-contained (own parse, no semantic index), and not part of
    // the package namespace.
    if crate::fs::has_rmd_extension(rel) {
        let lint = get_checks_rmd(&contents, rel, config)
            .with_context(|| format!("Failed to get checks for file: {}", rel.display()));
        return output(None, LintOutcome::Final(lint));
    }

    // Auto-generated files are never linted, but a generated file in `R/` still
    // defines package-namespace names that siblings may read.
    let generated = crate::fs::looks_generated(&contents);

    let parsed = air_r_parser::parse(&contents, RParserOptions::default());
    if parsed.has_error() {
        // A file we can't parse contributes no cross-file uses. Generated files
        // are reported as clean (the two-pass path skipped them before parsing);
        // otherwise raise the same parse error `get_checks` would.
        let lint: Result<Vec<Diagnostic>, anyhow::Error> = if generated {
            Ok(Vec::new())
        } else {
            Err(ParseError { filename: rel.to_path_buf() }.into())
        };
        return output(
            None,
            LintOutcome::Final(
                lint.with_context(|| format!("Failed to get checks for file: {}", rel.display())),
            ),
        );
    }

    let semantic =
        oak_semantic::build_index(&parsed.tree(), jarl_semantic::JarlImportsResolver::new(rel));

    let file_uses = extract.then(|| crate::db::collect_file_uses(rel.to_path_buf(), &semantic));

    if generated {
        return output(file_uses, LintOutcome::Final(Ok(Vec::new())));
    }

    let syntax = parsed.syntax();
    let expressions = parsed.tree().expressions();

    // Defer the cross-file `unused_object` decision exactly when cross-file
    // resolution is in play; otherwise the file is independent and finalizes
    // inline.
    let defer = extract;
    let core = run_checks_core(
        &contents,
        &syntax,
        &expressions,
        rel,
        config,
        pkg,
        pkg_contexts,
        file_pkg_info,
        &semantic,
        defer,
    )
    .with_context(|| format!("Failed to get checks for file: {}", rel.display()));
    let (mut checker, new_lines) = match core {
        Ok(value) => value,
        Err(e) => return output(file_uses, LintOutcome::Final(Err(e))),
    };

    if !defer {
        // `check_document` already filtered suppressions for this file.
        let diagnostics = finalize_diagnostics(
            std::mem::take(&mut checker.diagnostics),
            &checker.rule_set,
            config,
            rel,
            &new_lines,
        );
        return output(file_uses, LintOutcome::Final(Ok(diagnostics)));
    }

    let outdated_enabled = checker.is_rule_enabled(Rule::OutdatedSuppression);
    let deferred = DeferredLint {
        diagnostics: std::mem::take(&mut checker.diagnostics),
        provisional: std::mem::take(&mut checker.deferred_unused_object),
        suppression: checker.suppression.filter_snapshot(),
        new_lines,
        rule_set: checker.rule_set.clone(),
        outdated_enabled,
    };
    output(file_uses, LintOutcome::Deferred(Box::new(deferred)))
}

/// Parse a sibling file once and read off its cross-file contribution. Mirrors
/// the per-file work of [`crate::db::AnalysisDb::cross_file_used_objects`];
/// files that fail to parse contribute nothing.
fn extract_file_uses(path: &Path) -> Option<FileUses> {
    let source = fs::read_to_string(path).ok()?;
    let parsed = air_r_parser::parse(&source, RParserOptions::default());
    if parsed.has_error() {
        return None;
    }
    let index = oak_semantic::build_index(
        &parsed.tree(),
        jarl_semantic::JarlImportsResolver::new(path),
    );
    Some(crate::db::collect_file_uses(path.to_path_buf(), &index))
}

/// Phase 2 for one target: drop top-level `unused_object` diagnostics whose
/// object is read by a sibling, then run the suppression filtering and location
/// steps that `check_document`/`get_checks` would otherwise have done inline.
fn finalize_target(
    lint: LintOutcome,
    rel: &Path,
    config: &Config,
    cross_file_used: &HashMap<PathBuf, HashSet<String>>,
) -> Result<Vec<Diagnostic>, anyhow::Error> {
    let deferred = match lint {
        LintOutcome::Final(result) => return result,
        LintOutcome::Deferred(deferred) => *deferred,
    };

    // Keep provisional top-level diagnostics only for objects no sibling reads.
    // This must happen before suppression filtering so a suppression on a
    // cross-file-used object is still seen as outdated.
    let used = cross_file_used.get(rel);
    let mut diagnostics = deferred.diagnostics;
    for (name, diagnostic) in deferred.provisional {
        if !used.is_some_and(|names| names.contains(&name)) {
            diagnostics.push(diagnostic);
        }
    }

    let mut suppression = deferred.suppression;
    let mut diagnostics = suppression.filter_diagnostics(diagnostics);
    if deferred.outdated_enabled {
        let unused = suppression.get_unused_suppressions();
        diagnostics.extend(outdated_suppression(&unused));
    }

    Ok(finalize_diagnostics(
        diagnostics,
        &deferred.rule_set,
        config,
        rel,
        &deferred.new_lines,
    ))
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

/// Filter `config.rules_to_apply` down to the rules that apply to `path` after
/// accounting for `[lint.per-file-ignores]`.
fn effective_rules_for_file(config: &Config, path: &Path) -> RuleSet {
    if config.per_file_ignores.is_empty() {
        return config.rules_to_apply.clone();
    }
    let ignored = config.per_file_ignores.ignored_rules(path);
    config
        .rules_to_apply
        .iter()
        .filter(|rule| !ignored.contains(rule))
        .collect()
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

    // Files starting with "# Generated by" are ignored but they still
    // contribute use sites to cross-file analysis since the scan in
    // `make_package_analysis` runs independently.
    if crate::fs::looks_generated(&contents) {
        return Ok(Vec::new());
    }

    let checks = get_checks(
        &contents,
        &PathBuf::from(&path),
        &config,
        &pkg,
        &pkg_contexts,
        &file_pkg_info,
        // lint-only: on-disk contents match the cached index, so reuse it.
        true,
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

        // Skip auto-generated files: no diagnostics, no fixes.
        if crate::fs::looks_generated(&contents) {
            return Ok(Vec::new());
        }

        checks = get_checks(
            &contents,
            &PathBuf::from(&path),
            &config,
            &pkg,
            &pkg_contexts,
            &file_pkg_info,
            // Fix mode rewrites the file between iterations, so the on-disk
            // contents (and the index the pre-pass cached from them) drift from
            // the in-memory `contents`; rebuild the index rather than reuse it.
            false,
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
    use_cached_index: bool,
) -> Result<Vec<Diagnostic>> {
    if crate::fs::has_rmd_extension(file) {
        return get_checks_rmd(contents, file, config);
    }

    let parser_options = RParserOptions::default();
    let parsed = air_r_parser::parse(contents, parser_options);

    if parsed.has_error() {
        return Err(ParseError { filename: file.to_path_buf() }.into());
    }

    let syntax = parsed.syntax();
    let expressions = parsed.tree().expressions();

    // Build the semantic index for use-def-based rules. `source("path")`
    // calls inject `DefinitionKind::Import` entries via JarlImportsResolver;
    // the complementary "names read by sourced files" path is still handled
    // inside `SemanticInfo`.
    //
    // When `unused_object` runs in fix mode, the cross-file pre-pass already
    // built this file's index (with the same resolver) and stored it in
    // `pkg.file_indices`; reuse it rather than rebuilding. The pre-pass reads
    // from disk, so the cache is only valid before the first fix is applied —
    // fix mode rewrites the file between passes, so it always rebuilds from the
    // in-memory contents. (The lint-only path no longer goes through here; it
    // uses the fused single-parse pass.)
    //
    // Building (when not cached) happens here, in the parallel per-file pass,
    // rather than via the shared `AnalysisDb`: oak's salsa database is `Send`
    // but not `Sync`, so it can't be borrowed across rayon worker threads.
    let owned_semantic;
    let semantic: &oak_semantic::semantic_index::SemanticIndex = match use_cached_index
        .then(|| pkg.file_indices.get(file))
        .flatten()
    {
        Some(cached) => cached,
        None => {
            owned_semantic = oak_semantic::build_index(
                &parsed.tree(),
                jarl_semantic::JarlImportsResolver::new(file),
            );
            &owned_semantic
        }
    };

    let (mut checker, loc_new_lines) = run_checks_core(
        contents,
        &syntax,
        &expressions,
        file,
        config,
        pkg,
        pkg_contexts,
        file_pkg_info,
        semantic,
        false,
    )?;

    Ok(finalize_diagnostics(
        std::mem::take(&mut checker.diagnostics),
        &checker.rule_set,
        config,
        file,
        &loc_new_lines,
    ))
}

/// Run every per-file check on an already-parsed file and return the populated
/// `Checker` together with the file's newline offsets.
///
/// Shared by [`get_checks`] (fix mode / LSP) and the fused lint-only pass. When
/// `defer` is set, `unused_object` routes its top-level diagnostics into
/// `checker.deferred_unused_object` and `check_document` skips suppression
/// filtering, so the caller can drop cross-file-used objects and finalize once
/// every file in the package has been parsed.
#[allow(clippy::too_many_arguments)]
fn run_checks_core(
    contents: &str,
    syntax: &RSyntaxNode,
    expressions: &RExpressionList,
    file: &Path,
    config: &Config,
    pkg: &PackageAnalysis,
    pkg_contexts: &HashMap<PathBuf, PackageContext>,
    file_pkg_info: &HashMap<PathBuf, FilePackageInfo>,
    semantic: &oak_semantic::semantic_index::SemanticIndex,
    defer: bool,
) -> Result<(Checker, Vec<usize>)> {
    let suppression = SuppressionManager::from_node(syntax, contents);

    let mut checker = Checker::new(suppression, config.rule_options.clone());
    // Drop any rules ignored for this file via `[lint.per-file-ignores]`.
    checker.rule_set = effective_rules_for_file(config, file);
    checker.minimum_r_version = config.minimum_r_version;
    checker.defer_finalization = defer;
    checker.file_path = file.to_path_buf();

    // Wire up package context for package-specific rules.
    get_package_info(
        &mut checker,
        file,
        semantic,
        config,
        pkg_contexts,
        file_pkg_info,
    );

    // Look up per-file data from PackageAnalysis
    let package_file = PackageFileAnalysis::for_file(pkg, file);

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
        syntax,
        &mut checker,
        &package_file,
        Some(semantic),
    )?;

    let loc_new_lines = find_new_lines(syntax)?;
    Ok((checker, loc_new_lines))
}

/// Strip fixes that the user disabled and resolve each diagnostic's `(row, col)`
/// location.
///
/// Some rules carry a fix in their implementation even when the config disables
/// it (via the `unfixable`/`fixable` settings or a rule that is never fixable);
/// this clears those before `apply_fixes` runs. Locations are computed from
/// precomputed newline offsets so the fused lint-only pass — which no longer
/// holds the syntax tree by the time it finalizes — can call this with offsets
/// captured earlier.
fn finalize_diagnostics(
    diagnostics: Vec<Diagnostic>,
    rule_set: &RuleSet,
    config: &Config,
    file: &Path,
    loc_new_lines: &[usize],
) -> Vec<Diagnostic> {
    let rules_without_fix = rule_set
        .iter()
        .filter(|x| x.has_no_fix())
        .map(|x| x.name().to_string())
        .collect::<Vec<String>>();

    let diagnostics: Vec<Diagnostic> = diagnostics
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

    compute_lints_location(diagnostics, loc_new_lines)
}

/// Populate package context on the checker from pre-computed data.
///
/// For files inside an R package, copies the pre-computed `PackageContext`
/// fields. For scripts, harvests `library()`/`require()` calls from the
/// semantic index.
fn get_package_info(
    checker: &mut Checker,
    file: &Path,
    semantic: &oak_semantic::semantic_index::SemanticIndex,
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
            packages.extend(top_level_attached_packages(semantic));
            checker.loaded_packages = packages;
        }
    }
    checker.package_cache = config.package_cache.clone();
}

/// Collect package names from top-level `library()`/`require()` calls in
/// load order, deduplicated. Calls inside nested function bodies are
/// excluded because their attachment isn't statically guaranteed; calls
/// inside top-level `if`/loops are included because oak scopes them to the
/// file (R sequential execution makes their effect visible to subsequent
/// top-level code if the branch runs).
fn top_level_attached_packages(
    semantic: &oak_semantic::semantic_index::SemanticIndex,
) -> Vec<String> {
    use oak_semantic::semantic_index::SemanticCallKind;
    let top_level = oak_semantic::ScopeId::from(0);
    let mut out: Vec<String> = Vec::new();
    for call in semantic.semantic_calls() {
        if call.scope() != top_level {
            continue;
        }
        if let SemanticCallKind::Attach { package } = call.kind()
            && !out.iter().any(|p| p == package)
        {
            out.push(package.clone());
        }
    }
    out
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
        let syntax = parsed.syntax();
        let suppression = SuppressionManager::from_node(&syntax, &chunk.code);
        let has_suppressions = suppression.has_any_suppressions;
        let mut checker = Checker::new(suppression, config.rule_options.clone());
        checker.rule_set = effective_rules_for_file(config, file);
        checker.minimum_r_version = config.minimum_r_version;

        for expr in expressions {
            check_expression(&expr, &mut checker)?;
        }

        // Only run document-level checks if the examples code has inline
        // suppression comments. Most examples don't, and check_document is
        // otherwise unnecessary here (no package-level analysis, no
        // suppression-related diagnostics to report).
        if has_suppressions {
            check_document(
                expressions,
                &syntax,
                &mut checker,
                &PackageFileAnalysis::default(),
                None,
            )?;
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

    let syntax = parsed.syntax();
    let suppression = SuppressionManager::from_node(&syntax, &virtual_source);
    let mut checker = Checker::new(suppression, config.rule_options.clone());
    checker.rule_set = effective_rules_for_file(config, file);
    checker.minimum_r_version = config.minimum_r_version;

    let expressions = &parsed.tree().expressions();
    for expr in expressions {
        check_expression(&expr, &mut checker)?;
    }
    // check_document runs suppression filtering internally, so
    // checker.diagnostics is the post-suppression list after this call.
    // Rmd chunks don't participate in package-level analysis.
    check_document(
        expressions,
        &syntax,
        &mut checker,
        &PackageFileAnalysis::default(),
        None,
    )?;

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
