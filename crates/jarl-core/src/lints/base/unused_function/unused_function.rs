use air_r_syntax::{RBinaryExpression, RSyntaxKind, RSyntaxNode};
use biome_rowan::{AstNode, TextRange};
use jarl_semantic::assignment_rhs_is_function_def;
use oak_core::syntax_ext::RIdentifierExt;
use oak_semantic::semantic_index::{DefinitionKind, ScopeId, SemanticIndex};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::namespace::Namespace;
use crate::package::{FileScope, SharedFileData};
use crate::utils::scan_symbols;

/// <!-- docs: start -->
/// Version added: 0.5.0
///
/// ## What it does
///
/// Checks for unused functions in R packages. It looks for:
///
/// - Functions defined in `R/` that are not exported and not used anywhere in
///   the package (including `R/`, `inst/tinytest/`, `inst/tests/`, `src/`, and
///   `tests/`).
/// - Functions defined in `tests/` that are not used anywhere in `tests/`.
/// - Functions defined in `inst/tinytest/` or `inst/tests/` that are not used
///   anywhere within that directory.
///
/// ## Why is this bad?
///
/// Functions must be documented in NAMESPACE to be exported to end users. A
/// function that is never called nor exported is likely dead code left over
/// from refactoring. Removing unused internal functions keeps the codebase
/// easier to understand and maintain.
///
/// ## Limitations
///
/// There are many ways to call a function in R code (e.g. `foo()`,
/// `do.call("foo", ...)`, `lapply(x, foo)` among others). Jarl tries to limit
/// false positives as much as possible, at the expense of false negatives. This
/// means that reporting a function that is actually used somewhere (false positive)
/// is considered a bug, but not reporting a function that isn't used anywhere
/// (false negative) isn't considered a bug (but can be suggested as a feature
/// request).
///
/// ## Example
///
/// ```r
/// # In NAMESPACE: export(public_fn)
///
/// # In R/public.R:
/// public_fn <- function(x) {
///   check_character(x)
/// }
///
/// # In R/helper.R:
/// check_character <- function(x) {
///   stopifnot(is.character(x))
/// }
/// check_length <- function(x, y) {
///   stopifnot(length(x) == y)
/// }
///
/// # `public_fn()` is exported by the package, so it is considered used.
/// # `check_character()` isn't exported but used in `public_fn`.
/// # `check_length()` isn't exported but and isn't used anywhere, so it is
/// # reported.
/// ```
/// <!-- docs: end -->
// ## Implementation
//
// Operates on the already-scanned `SharedFileData` of a package rather than
// reading its files again. Files in R/ are judged on the names their syntax
// tree reads (see `function_reads`), other files and files that don't parse on
// the words of their text.
//
// `namespaces` maps package root paths to their parsed NAMESPACE file.
// Packages without a NAMESPACE entry are skipped.
pub(crate) fn compute_unused_from_shared(
    shared_data: &[SharedFileData],
    options: &crate::lints::base::unused_function::options::ResolvedUnusedFunctionOptions,
    namespaces: &HashMap<PathBuf, Namespace>,
) -> HashMap<PathBuf, Vec<(String, TextRange, String)>> {
    // Group by package root
    let mut packages: HashMap<&str, Vec<&SharedFileData>> = HashMap::new();
    for fd in shared_data {
        packages.entry(&fd.root_key).or_default().push(fd);
    }

    let mut result: HashMap<PathBuf, Vec<(String, TextRange, String)>> = HashMap::new();

    for (_root_key, file_data) in packages {
        // Separate files by scope.
        let r_files: Vec<&&SharedFileData> = file_data
            .iter()
            .filter(|f| f.scope == FileScope::R)
            .collect();
        let extra_files: Vec<&&SharedFileData> = file_data
            .iter()
            .filter(|f| f.scope != FileScope::R)
            .collect();
        let tests_files: Vec<&&SharedFileData> = file_data
            .iter()
            .filter(|f| f.scope == FileScope::Tests)
            .collect();
        let inst_files: Vec<&&SharedFileData> = file_data
            .iter()
            .filter(|f| f.scope == FileScope::Inst)
            .collect();

        // ── R scope ────────────────────────────────

        // Collect ALL defined function names across R/ (for exportPattern matching)
        let all_defined_names: Vec<String> = r_files
            .iter()
            .flat_map(|f| f.assignments.iter().map(|(name, _, _, _)| name.clone()))
            .collect();
        let all_defined_name_refs: Vec<&str> =
            all_defined_names.iter().map(|s| s.as_str()).collect();

        let Some(first) = r_files.first() else {
            continue;
        };
        let Some(namespace) = namespaces.get(&first.package_root) else {
            continue;
        };
        let namespace_exports = namespace.exported(all_defined_name_refs.iter().copied());

        // Names read anywhere in R/.
        let mut read_names: HashSet<&str> = HashSet::new();
        for file in &r_files {
            if let Some(reads) = &file.function_reads {
                read_names.extend(reads.iter().map(String::as_str));
                continue;
            }
            // Without a syntax tree, fall back to counting words: a name is
            // read if it appears more often than this file defines it.
            let mut definitions: HashMap<&str, usize> = HashMap::new();
            for (name, _, _, _) in &file.assignments {
                *definitions.entry(name.as_str()).or_insert(0) += 1;
            }
            read_names.extend(
                file.symbol_counts
                    .iter()
                    .filter(|(name, count)| {
                        **count > definitions.get(name.as_str()).copied().unwrap_or(0)
                    })
                    .map(|(name, _)| name.as_str()),
            );
        }

        // Symbols from extra files (tests/, inst/tinytest/, inst/tests/, src/).
        let extra_symbol_set: HashSet<&str> = extra_files
            .iter()
            .flat_map(|f| f.symbol_counts.keys().map(|s| s.as_str()))
            .collect();

        // R package hook functions that are called by the runtime, not by
        // user code. These are typically defined in `zzz.R` and should never
        // be flagged as unused.
        let package_hooks: HashSet<&str> = HashSet::from([
            ".onLoad",
            "on_load",
            ".onAttach",
            ".onDetach",
            ".onUnload",
            ".Last.lib",
            ".First.lib",
        ]);

        // All symbols across R/ files (used for S3 method heuristic).
        let all_symbols: HashSet<&str> = read_names
            .iter()
            .copied()
            .chain(all_defined_name_refs.iter().copied())
            .collect();

        for file in &r_files {
            let mut unused: Vec<(String, TextRange, String)> = Vec::new();

            for (name, range, line, col) in &file.assignments {
                // Skip exported functions
                if namespace_exports.contains(name) {
                    continue;
                }

                // Skip R package hook functions (.onLoad, .onAttach, etc.)
                if package_hooks.contains(name.as_str()) {
                    continue;
                }

                // Skip functions matching user-configured patterns
                if options.is_skipped(name) {
                    continue;
                }

                // Skip probable internal S3 methods. If a function name
                // contains a dot, it may be an S3 method dispatched implicitly
                // (e.g. `print.myclass` is called when `print()` runs on an
                // object of class "myclass"). Since class names can contain
                // dots (e.g. `data.table`), we try every split point: for
                // `sort_by.data.table` we check if `sort_by` or `sort_by.data`
                // appears as a symbol in the package. If any prefix matches,
                // assume it could be S3 dispatch and skip.
                if name.contains('.') {
                    let is_probable_s3 = name
                        .match_indices('.')
                        .any(|(pos, _)| all_symbols.contains(&name[..pos]));
                    if is_probable_s3 {
                        continue;
                    }
                }

                if !read_names.contains(name.as_str()) && !extra_symbol_set.contains(name.as_str())
                {
                    let help = format!(
                        "Defined at {path}:{line}:{col}.",
                        path = file.rel_path.display()
                    );
                    unused.push((name.clone(), *range, help));
                }
            }

            if !unused.is_empty() {
                result.insert(file.rel_path.clone(), unused);
            }
        }

        // ── Tests and Inst scopes ───────────────────────────────
        // A function defined in one of these directories is unused if it
        // doesn't appear in any other file within that same scope. No
        // NAMESPACE export check is needed.

        for scope_files in [&tests_files, &inst_files] {
            if scope_files.is_empty() {
                continue;
            }

            // Total symbol occurrences within this scope.
            let mut scope_occurrences: HashMap<&str, usize> = HashMap::new();
            for file in scope_files {
                for (name, count) in &file.symbol_counts {
                    *scope_occurrences.entry(name.as_str()).or_insert(0) += count;
                }
            }

            // Total definitions within this scope.
            let mut scope_definitions: HashMap<&str, usize> = HashMap::new();
            for file in scope_files {
                for (name, _, _, _) in &file.assignments {
                    *scope_definitions.entry(name.as_str()).or_insert(0) += 1;
                }
            }

            for file in scope_files {
                let mut unused: Vec<(String, TextRange, String)> = Vec::new();

                for (name, range, line, col) in &file.assignments {
                    // Skip functions matching user-configured patterns
                    if options.is_skipped(name) {
                        continue;
                    }

                    let occurrences = scope_occurrences.get(name.as_str()).copied().unwrap_or(0);
                    let definitions = scope_definitions.get(name.as_str()).copied().unwrap_or(0);

                    if occurrences <= definitions {
                        let scope_dir = scope_dir_from_path(&file.rel_path);
                        let help = format!(
                            "Defined at {path}:{line}:{col} but never called in {scope_dir}",
                            path = file.rel_path.display()
                        );
                        unused.push((name.clone(), *range, help));
                    }
                }

                if !unused.is_empty() {
                    result
                        .entry(file.rel_path.clone())
                        .or_default()
                        .extend(unused);
                }
            }
        }
    }

    result
}

/// The names a parsed R file reads, i.e. the names that may call one of the
/// package's functions.
///
/// Every identifier counts unless its position can't refer to a function of
/// the package (#483): an argument or parameter name, a member name (`x$foo`),
/// the package of `pkg::foo`, an assignment target, or a read that resolves to
/// a parameter or local of an enclosing function. Everything else is kept, so
/// that quoted code (`quote(foo())`) still counts.
///
/// Words inside strings and roxygen comments also count, since they can name a
/// function indirectly (`do.call("foo", ...)`, `parse(text = "foo()")`,
/// `\Sexpr`).
pub(crate) fn function_reads(
    source: &str,
    root: &RSyntaxNode,
    index: &SemanticIndex,
) -> HashSet<String> {
    let file_scope = ScopeId::from(0);
    let mut local_reads: HashSet<TextRange> = HashSet::new();
    for scope in index.scope_ids() {
        if scope == file_scope {
            continue;
        }
        for (use_id, use_site) in index.uses(scope).iter() {
            // A local function definition is not excluded: it may be one of
            // the definitions this rule checks, e.g. in a file wrapped in an
            // unindented `local({ ... })`.
            let is_local = index.use_is_bound(scope, use_id)
                && index.reaching_definitions(scope, use_id).into_iter().all(
                    |(def_scope, def_id)| {
                        def_scope != file_scope
                            && match index.definitions(def_scope)[def_id].kind() {
                                DefinitionKind::Parameter(_) | DefinitionKind::ForVariable(_) => {
                                    true
                                }
                                DefinitionKind::Assignment(ptr) => {
                                    !assignment_rhs_is_function_def(&ptr.to_node(root))
                                }
                                _ => false,
                            }
                    },
                );
            if is_local {
                local_reads.insert(use_site.range());
            }
        }
    }

    let mut reads: HashSet<String> = HashSet::new();
    for node in root.descendants() {
        match node.kind() {
            RSyntaxKind::R_IDENTIFIER if is_function_read(&node, &local_reads) => {
                if let Some(ident) = air_r_syntax::RIdentifier::cast(node) {
                    reads.insert(ident.name_text());
                }
            }
            RSyntaxKind::R_STRING_VALUE => {
                reads.extend(scan_symbols(&node.text_trimmed().to_string()).into_keys());
            }
            _ => {}
        }
    }

    let roxygen: Vec<&str> = source
        .lines()
        .filter(|line| line.trim_start().starts_with("#'"))
        .collect();
    reads.extend(scan_symbols(&roxygen.join("\n")).into_keys());

    reads
}

fn is_function_read(node: &RSyntaxNode, local_reads: &HashSet<TextRange>) -> bool {
    let Some(parent) = node.parent() else {
        return true;
    };
    let is_first_child = parent.first_child().as_ref() == Some(node);
    match parent.kind() {
        RSyntaxKind::R_ARGUMENT_NAME_CLAUSE | RSyntaxKind::R_PARAMETER => return false,
        // `x@foo` is a slot, but `$` stays a use: `asNamespace("pkg")$foo()`
        // and `env$foo()` call package functions.
        RSyntaxKind::R_EXTRACT_EXPRESSION if !is_first_child && is_slot_access(&parent) => {
            return false;
        }
        RSyntaxKind::R_NAMESPACE_EXPRESSION if is_first_child => return false,
        // Function lookup skips non-function bindings, so `foo()` calls the
        // package's `foo` even where a local `foo <- 1` is in scope.
        RSyntaxKind::R_CALL if is_first_child => return true,
        RSyntaxKind::R_BINARY_EXPRESSION if is_assignment_target(&parent, node) => return false,
        _ => {}
    }
    !local_reads.contains(&node.text_trimmed_range())
}

fn is_slot_access(parent: &RSyntaxNode) -> bool {
    air_r_syntax::RExtractExpression::cast(parent.clone())
        .and_then(|extract| extract.operator().ok())
        .is_some_and(|operator| operator.text_trimmed() == "@")
}

fn is_assignment_target(parent: &RSyntaxNode, node: &RSyntaxNode) -> bool {
    let Some(bin) = RBinaryExpression::cast(parent.clone()) else {
        return false;
    };
    let Ok(operator) = bin.operator() else {
        return false;
    };
    let target = match operator.text_trimmed() {
        "<-" | "<<-" | "=" => bin.left(),
        "->" | "->>" => bin.right(),
        _ => return false,
    };
    target.is_ok_and(|target| target.syntax() == node)
}

/// Recursively collect files under `dir` that match `predicate`.
pub(crate) fn collect_files(dir: &Path, predicate: fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = match std::fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if predicate(&path) {
                files.push(path);
            }
        }
    }
    files
}

pub(crate) fn has_cpp_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("c" | "cpp" | "h" | "hpp")
    )
}

/// Extract a human-readable scope directory from a file path, e.g.
/// `"tests/"` or `"inst/tests/"`. Used in help messages.
fn scope_dir_from_path(rel_path: &Path) -> String {
    let components: Vec<_> = rel_path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    for (i, comp) in components.iter().enumerate() {
        if comp == "tests" {
            return "tests/".to_string();
        }
        if comp == "inst"
            && let Some(next) = components.get(i + 1)
            && (next == "tinytest" || next == "tests")
        {
            return format!("inst/{next}/");
        }
    }
    "tests/".to_string()
}
