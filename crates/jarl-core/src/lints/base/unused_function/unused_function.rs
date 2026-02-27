use biome_rowan::TextRange;
use rayon::prelude::*;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::fs::has_r_extension;
use crate::lints::base::duplicated_function_definition::duplicated_function_definition::{
    is_in_r_package, scan_top_level_assignments,
};

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
    let mut symbols: HashMap<String, usize> = HashMap::new();

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
                *symbols.entry(name.to_string()).or_insert(0) += 1;
            } else {
                i += 1;
            }
        }
    }

    symbols
}

// (rel_path, abs_path, definitions, symbol_counts) — per file within a package
type PackageFileEntry = (
    PathBuf,
    PathBuf,
    Vec<(String, TextRange, u32, u32)>,
    HashMap<String, usize>,
);

// (package_root_key, rel_path, abs_path, definitions, symbol_counts)
type FileData = (
    String,
    PathBuf,
    PathBuf,
    Vec<(String, TextRange, u32, u32)>,
    HashMap<String, usize>,
);

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

/// Pre-compute unused internal functions across an R package.
///
/// Returns a map from relativized file path to a list of
/// `(name, lhs_range, help)` triples for functions that are:
/// 1. Defined as top-level assignments in `R/`
/// 2. Not exported in `NAMESPACE`
/// 3. Never appear as an identifier in any file in `R/`, `inst/tinytest/`, `tests/`, or `src/`
pub fn compute_package_unused_functions(
    paths: &[PathBuf],
    options: &crate::rule_options::unused_function::ResolvedUnusedFunctionOptions,
) -> HashMap<PathBuf, Vec<(String, TextRange, String)>> {
    // Step 1: collect data from each file in parallel
    let file_data: Vec<FileData> = paths
        .par_iter()
        .filter(|p| has_r_extension(p))
        .filter(|p| is_in_r_package(p).unwrap_or(false))
        .filter_map(|path| {
            let root = path.parent()?;
            let rel_path = PathBuf::from(crate::fs::relativize_path(path));
            let root_key = crate::fs::relativize_path(root);
            let content = std::fs::read_to_string(path).ok()?;
            let definitions = scan_top_level_assignments(&content);
            let calls = scan_symbols(&content);
            Some((root_key, rel_path, path.clone(), definitions, calls))
        })
        .collect();

    // Step 2: group by package root
    let mut packages: HashMap<String, Vec<PackageFileEntry>> = HashMap::new();
    for (root_key, rel_path, abs_path, definitions, calls) in file_data {
        packages
            .entry(root_key)
            .or_default()
            .push((rel_path, abs_path, definitions, calls));
    }

    let mut result: HashMap<PathBuf, Vec<(String, TextRange, String)>> = HashMap::new();

    for (_root_key, file_data) in packages {
        // Collect ALL defined function names across the package (for exportPattern matching)
        let all_defined_names: Vec<String> = file_data
            .iter()
            .flat_map(|(_, _, defs, _)| defs.iter().map(|(name, _, _, _)| name.clone()))
            .collect();
        let all_defined_name_refs: Vec<&str> =
            all_defined_names.iter().map(|s| s.as_str()).collect();

        // Step 3: parse NAMESPACE
        // The package root is the parent of the R/ directory. We can find it
        // from any abs_path: abs_path.parent() is R/, its parent is the root.
        let namespace_exports = if let Some(first_abs) = file_data.first().map(|(_, abs, _, _)| abs)
        {
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

        // Scan inst/tinytest, tests/, and src/ for symbol usage so that
        // internal functions referenced only from test, example, or C/C++
        // code are not flagged.
        let extra_symbols: Vec<HashMap<String, usize>> = {
            let package_root = file_data
                .first()
                .and_then(|(_, abs, _, _)| abs.parent())
                .and_then(|r_dir| r_dir.parent());
            let mut syms = Vec::new();
            if let Some(root) = package_root {
                // R files in inst/tinytest and tests/
                for dir_name in &["inst/tinytest", "tests"] {
                    let dir = root.join(dir_name);
                    if dir.is_dir() {
                        for file_path in collect_files(&dir, has_r_extension) {
                            if let Ok(content) = std::fs::read_to_string(&file_path) {
                                syms.push(scan_symbols(&content));
                            }
                        }
                    }
                }
                // C/C++ files in src/
                let src_dir = root.join("src");
                if src_dir.is_dir() {
                    for file_path in collect_files(&src_dir, has_cpp_extension) {
                        if let Ok(content) = std::fs::read_to_string(&file_path) {
                            syms.push(scan_symbols(&content));
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
            .flat_map(|(_, _, _, syms)| syms.keys().map(|s| s.as_str()))
            .collect();

        // Step 4: for each defined function, check if its name appears
        // anywhere else in the package. A name counts as "used" if:
        //   - it appears in any OTHER file as a symbol, OR
        //   - it appears MORE times than it is defined in the SAME file
        //     (i.e. the name is also referenced, not just assigned).
        for (i, (rel_path, _, definitions, sym_counts)) in file_data.iter().enumerate() {
            let mut unused: Vec<(String, TextRange, String)> = Vec::new();

            // Count how many times each name is defined in this file
            let mut def_counts: HashMap<String, usize> = HashMap::new();
            for (name, _, _, _) in definitions {
                *def_counts.entry(name.clone()).or_insert(0) += 1;
            }

            for (name, range, line, col) in definitions {
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

                // Used in another R/ file, in inst/tinytest, tests/, or src/?
                let used_in_other_file = file_data
                    .iter()
                    .enumerate()
                    .any(|(j, (_, _, _, syms))| j != i && syms.contains_key(name))
                    || extra_symbols.iter().any(|syms| syms.contains_key(name));

                // Used in the same file beyond its own definition(s)?
                // Each definition contributes exactly one occurrence of the
                // name to the symbol count (the LHS). If the total count
                // exceeds the number of definitions, the name is also
                // referenced elsewhere in the file.
                let n_defs = def_counts.get(name).copied().unwrap_or(0);
                let n_occurrences = sym_counts.get(name).copied().unwrap_or(0);
                let used_in_same_file = n_occurrences > n_defs;

                if !used_in_other_file && !used_in_same_file {
                    let help = format!(
                        "Defined at {path}:{line}:{col} but never called",
                        path = rel_path.display()
                    );
                    unused.push((name.clone(), *range, help));
                }
            }

            if !unused.is_empty() {
                result.insert(rel_path.clone(), unused);
            }
        }
    }

    result
}
