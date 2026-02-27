use biome_rowan::TextRange;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::fs::has_r_extension;
use crate::package::SharedFileData;

/// ## What it does
///
/// Checks for unused functions, currently limited to R packages. It looks for
/// functions defined in the `R` folder that are not exported and not used
/// anywhere in the package (including the `R`, `inst/tinytest`, `src`, and
/// `tests` folders).
///
/// ## Why is this bad?
///
/// An internal function that is never called is likely dead code left over from
/// refactoring. Removing it keeps the codebase easier to understand and
/// maintain.
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
fn extract_directive<'a>(line: &'a str, directive: &str) -> Option<&'a str> {
    // Find a NAMESPACE directive (e.g. `S3method`, `export`) in a line and
    // return its parenthesized arguments. Handles lines where the directive is
    // preceded by an `if (...)` guard, e.g.:
    //   `if (getRversion() >= "4.4.0") S3method(sort_by, data.table)`

    // Find `directive(` in the line
    let dir_with_paren = format!("{directive}(");
    let start = line.find(&dir_with_paren)?;
    let args_start = start + dir_with_paren.len();

    // Make sure the directive is not part of a longer word
    // (e.g. "exportPattern" should not match "export")
    if start > 0 {
        let prev = line.as_bytes()[start - 1];
        if prev.is_ascii_alphanumeric() || prev == b'_' {
            return None;
        }
    }

    // Find the matching closing paren
    let rest = &line[args_start..];
    let end = rest.rfind(')')?;
    Some(&rest[..end])
}

/// Parse a NAMESPACE file and return the set of exported function names.
///
/// Handles both `export(name)` directives and `exportPattern(regex)` directives.
/// For `exportPattern`, the regex is compiled and matched against `all_names`
/// to expand it into concrete names.
pub fn parse_namespace_exports(content: &str, all_names: &[&str]) -> HashSet<String> {
    let mut exports = HashSet::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // NAMESPACE directives may be wrapped in `if (...)` guards, e.g.
        //   if (getRversion() >= "4.4.0") S3method(sort_by, data.table)
        // We extract the inner `directive(...)` by finding the directive
        // keyword anywhere in the line.
        for directive in [
            "export",
            "exportPattern",
            "S3method",
            "exportMethods",
            "exportClasses",
        ] {
            if let Some(inner) = extract_directive(trimmed, directive) {
                match directive {
                    "export" => {
                        for name in inner.split(',') {
                            let name = name.trim().trim_matches('"').trim_matches('\'');
                            if !name.is_empty() {
                                exports.insert(name.to_string());
                            }
                        }
                    }
                    "exportPattern" => {
                        let pattern = inner.trim().trim_matches('"').trim_matches('\'');
                        if let Ok(re) = Regex::new(pattern) {
                            for &name in all_names {
                                if re.is_match(name) {
                                    exports.insert(name.to_string());
                                }
                            }
                        }
                    }
                    "S3method" => {
                        // S3method(generic, class) or
                        // S3method(pkg::generic, class) or
                        // S3method(generic, class, method_fn)
                        let parts: Vec<&str> = inner.splitn(4, ',').collect();
                        if parts.len() >= 2 {
                            let raw_generic = parts[0].trim().trim_matches('"').trim_matches('\'');
                            let class = parts[1].trim().trim_matches('"').trim_matches('\'');

                            // Strip optional pkg:: or pkg::: prefix
                            let generic = raw_generic
                                .rsplit_once("::")
                                .map(|(_, g)| g)
                                .unwrap_or(raw_generic);

                            if parts.len() >= 3 {
                                let method_fn =
                                    parts[2].trim().trim_matches('"').trim_matches('\'');
                                if !method_fn.is_empty() {
                                    exports.insert(method_fn.to_string());
                                }
                            } else {
                                exports.insert(format!("{generic}.{class}"));
                            }
                        }
                    }
                    "exportMethods" | "exportClasses" => {
                        for name in inner.split(',') {
                            let name = name.trim().trim_matches('"').trim_matches('\'');
                            if !name.is_empty() {
                                exports.insert(name.to_string());
                            }
                        }
                    }
                    _ => {}
                }
                break;
            }
        }
    }

    exports
}

/// Scan source text for all R-style identifiers (symbols).
///
/// Returns a map from identifier name to occurrence count. This intentionally
/// over-counts (e.g. it will match inside comments and strings) — that is fine
/// because false negatives (failing to flag truly unused functions) are
/// preferable to false positives. By collecting all symbols rather than just
/// `name(` call patterns, we also cover indirect references like
/// `do.call("name", ...)`, `lapply(xs, name)`, `match.fun(name)`, etc.
pub fn scan_symbols(content: &str) -> HashMap<String, usize> {
    let mut symbols: HashMap<&str, usize> = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim_start();
        // Skip regular comments but keep roxygen comments (#') since they
        // may reference internal functions (e.g. via \Sexpr).
        if trimmed.starts_with('#') && !trimmed.starts_with("#'") {
            continue;
        }

        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let b = bytes[i];

            // R identifiers start with a letter, `.`, or `_`
            if b.is_ascii_alphabetic() || b == b'.' || b == b'_' {
                let start = i;
                i += 1;
                while i < len
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'.' || bytes[i] == b'_')
                {
                    i += 1;
                }
                let name = &line[start..i];
                *symbols.entry(name).or_insert(0) += 1;
            } else {
                i += 1;
            }
        }
    }

    symbols
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect()
}

/// Recursively collect files under `dir` that match `predicate`.
fn collect_files(dir: &Path, predicate: fn(&Path) -> bool) -> Vec<PathBuf> {
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

fn has_cpp_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("c" | "cpp" | "h" | "hpp")
    )
}

/// Compute unused functions from pre-scanned shared file data.
///
/// This is the inner logic extracted from `compute_package_unused_functions`,
/// operating on already-scanned `SharedFileData` to avoid redundant file reads.
/// Uses O(1) cross-file symbol lookup via pre-computed frequency maps.
pub(crate) fn compute_unused_from_shared(
    shared_data: &[SharedFileData],
    options: &crate::rule_options::unused_function::ResolvedUnusedFunctionOptions,
) -> HashMap<PathBuf, Vec<(String, TextRange, String)>> {
    // Group by package root
    let mut packages: HashMap<&str, Vec<&SharedFileData>> = HashMap::new();
    for fd in shared_data {
        packages.entry(&fd.root_key).or_default().push(fd);
    }

    let mut result: HashMap<PathBuf, Vec<(String, TextRange, String)>> = HashMap::new();

    for (_root_key, file_data) in packages {
        // Collect ALL defined function names across the package (for exportPattern matching)
        let all_defined_names: Vec<String> = file_data
            .iter()
            .flat_map(|f| f.assignments.iter().map(|(name, _, _, _)| name.clone()))
            .collect();
        let all_defined_name_refs: Vec<&str> =
            all_defined_names.iter().map(|s| s.as_str()).collect();

        // Parse NAMESPACE
        // The package root is the parent of the R/ directory. We can find it
        // from any abs_path: abs_path.parent() is R/, its parent is the root.
        let namespace_exports = if let Some(first_abs) = file_data.first().map(|f| &f.abs_path) {
            let package_root = first_abs.parent().and_then(|r_dir| r_dir.parent());
            if let Some(root) = package_root {
                let ns_path = root.join("NAMESPACE");
                if let Ok(ns_content) = std::fs::read_to_string(&ns_path) {
                    parse_namespace_exports(&ns_content, &all_defined_name_refs)
                } else {
                    // No NAMESPACE file — treat everything as potentially exported
                    // (skip this package)
                    continue;
                }
            } else {
                continue;
            }
        } else {
            continue;
        };

        // --- O(1) cross-file symbol lookup ---
        // Pre-compute: for each symbol name, how many R/ files contain it.
        let mut symbol_file_count: HashMap<&str, usize> = HashMap::new();
        for file in &file_data {
            for name in file.symbol_counts.keys() {
                *symbol_file_count.entry(name.as_str()).or_insert(0) += 1;
            }
        }

        // Scan inst/tinytest, tests/, and src/ for symbol usage so that
        // internal functions referenced only from test, example, or C/C++
        // code are not flagged. Build a single HashSet for O(1) lookup.
        let extra_symbol_set: HashSet<String> = {
            let package_root = file_data
                .first()
                .and_then(|f| f.abs_path.parent())
                .and_then(|r_dir| r_dir.parent());
            let mut syms = HashSet::new();
            if let Some(root) = package_root {
                // R files in inst/tinytest and tests/
                for dir_name in &["inst/tinytest", "tests"] {
                    let dir = root.join(dir_name);
                    if dir.is_dir() {
                        for file_path in collect_files(&dir, has_r_extension) {
                            if let Ok(content) = std::fs::read_to_string(&file_path) {
                                for (k, _) in scan_symbols(&content) {
                                    syms.insert(k);
                                }
                            }
                        }
                    }
                }
                // C/C++ files in src/
                let src_dir = root.join("src");
                if src_dir.is_dir() {
                    for file_path in collect_files(&src_dir, has_cpp_extension) {
                        if let Ok(content) = std::fs::read_to_string(&file_path) {
                            for (k, _) in scan_symbols(&content) {
                                syms.insert(k);
                            }
                        }
                    }
                }
            }
            syms
        };

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

        // Collect all symbols across all files in the package (used for
        // S3 method heuristic below).
        let all_symbols: HashSet<&str> = file_data
            .iter()
            .flat_map(|f| f.symbol_counts.keys().map(|s| s.as_str()))
            .collect();

        // For each defined function, check if its name appears anywhere else
        // in the package using the pre-computed frequency maps (O(1) per check).
        for file in &file_data {
            let mut unused: Vec<(String, TextRange, String)> = Vec::new();

            // Count how many times each name is defined in this file
            let mut def_counts: HashMap<&str, usize> = HashMap::new();
            for (name, _, _, _) in &file.assignments {
                *def_counts.entry(name.as_str()).or_insert(0) += 1;
            }

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

                // O(1) cross-file lookup: check symbol_file_count to see if
                // the name appears in other R/ files, and extra_symbol_set for
                // test/src files.
                let file_count = symbol_file_count.get(name.as_str()).copied().unwrap_or(0);
                let in_extra = extra_symbol_set.contains(name.as_str());
                let in_other_r_file =
                    file_count > 1 || (file_count == 1 && !file.symbol_counts.contains_key(name));
                let used_in_other_file = in_other_r_file || in_extra;

                // Used in the same file beyond its own definition(s)?
                // Each definition contributes exactly one occurrence of the
                // name to the symbol count (the LHS). If the total count
                // exceeds the number of definitions, the name is also
                // referenced elsewhere in the file.
                let n_defs = def_counts.get(name.as_str()).copied().unwrap_or(0);
                let n_occurrences = file.symbol_counts.get(name).copied().unwrap_or(0);
                let used_in_same_file = n_occurrences > n_defs;

                if !used_in_other_file && !used_in_same_file {
                    let help = format!(
                        "Defined at {path}:{line}:{col} but never called",
                        path = file.rel_path.display()
                    );
                    unused.push((name.clone(), *range, help));
                }
            }

            if !unused.is_empty() {
                result.insert(file.rel_path.clone(), unused);
            }
        }
    }

    result
}
