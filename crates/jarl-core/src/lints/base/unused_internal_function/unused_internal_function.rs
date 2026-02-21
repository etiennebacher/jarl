use biome_rowan::TextRange;
use rayon::prelude::*;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::lints::base::duplicated_function_definition::duplicated_function_definition::{
    is_in_r_package, scan_top_level_assignments,
};

// ## What it does
//
// Checks for internal (non-exported) functions in an R package that are never
// called anywhere in the package.
//
// ## Why is this bad?
//
// An internal function that is never called is likely dead code left over from
// refactoring. Removing it keeps the codebase easier to understand and
// maintain.
//
// This is a "dirty" first implementation: it looks for `name(` patterns in
// source text rather than performing full static analysis. Indirect call
// patterns like `do.call()`, `lapply()`, `match.fun()`, `Map()`, etc. are
// **not** detected, so there may be false positives for functions that are
// only invoked indirectly. The rule is therefore disabled by default.
//
// ## Example
//
// ```r
// # In NAMESPACE: export(public_fn)
//
// # In R/public.R:
// public_fn <- function() helper()
//
// # In R/helper.R:
// helper <- function() 1
//
// # `helper` is internal (not exported) and called by `public_fn`, so it is
// # fine. But if nothing ever called `helper`, it would be flagged.
// ```

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

        // export(name1, name2, ...)
        if let Some(inner) = trimmed
            .strip_prefix("export(")
            .and_then(|s| s.strip_suffix(')'))
        {
            for name in inner.split(',') {
                let name = name.trim().trim_matches('"').trim_matches('\'');
                if !name.is_empty() {
                    exports.insert(name.to_string());
                }
            }
            continue;
        }

        // exportPattern(regex)
        if let Some(inner) = trimmed
            .strip_prefix("exportPattern(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let pattern = inner.trim().trim_matches('"').trim_matches('\'');
            if let Ok(re) = Regex::new(pattern) {
                for &name in all_names {
                    if re.is_match(name) {
                        exports.insert(name.to_string());
                    }
                }
            }
            continue;
        }

        // S3method(generic, class) or S3method(pkg::generic, class) or
        // S3method(generic, class, method_fn)
        //
        // The function implementing the method is `generic.class` by default,
        // or `method_fn` if a third argument is provided.
        // When the generic is qualified with a package (`pkg::generic`), we
        // strip the namespace prefix to get the bare generic name.
        if let Some(inner) = trimmed
            .strip_prefix("S3method(")
            .and_then(|s| s.strip_suffix(')'))
        {
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
                    // Explicit method function name (third argument)
                    let method_fn = parts[2].trim().trim_matches('"').trim_matches('\'');
                    if !method_fn.is_empty() {
                        exports.insert(method_fn.to_string());
                    }
                } else {
                    exports.insert(format!("{generic}.{class}"));
                }
            }
            continue;
        }

        // exportMethods(name) — S4 method exports
        if let Some(inner) = trimmed
            .strip_prefix("exportMethods(")
            .and_then(|s| s.strip_suffix(')'))
        {
            for name in inner.split(',') {
                let name = name.trim().trim_matches('"').trim_matches('\'');
                if !name.is_empty() {
                    exports.insert(name.to_string());
                }
            }
            continue;
        }

        // exportClasses(name) — S4 class exports
        if let Some(inner) = trimmed
            .strip_prefix("exportClasses(")
            .and_then(|s| s.strip_suffix(')'))
        {
            for name in inner.split(',') {
                let name = name.trim().trim_matches('"').trim_matches('\'');
                if !name.is_empty() {
                    exports.insert(name.to_string());
                }
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
        if trimmed.starts_with('#') {
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

/// Pre-compute unused internal functions across an R package.
///
/// Returns a map from relativized file path to a list of
/// `(name, lhs_range, help)` triples for functions that are:
/// 1. Defined as top-level assignments in `R/`
/// 2. Not exported in `NAMESPACE`
/// 3. Never appear as an identifier in any file in the package
pub fn compute_package_unused_internal_functions(
    paths: &[PathBuf],
) -> HashMap<PathBuf, Vec<(String, TextRange, String)>> {
    // Step 1: collect data from each file in parallel
    let file_data: Vec<FileData> = paths
        .par_iter()
        .filter(|p| crate::fs::has_r_extension(p))
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

                // Used in another file?
                let used_in_other_file = file_data
                    .iter()
                    .enumerate()
                    .any(|(j, (_, _, _, syms))| j != i && syms.contains_key(name));

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
