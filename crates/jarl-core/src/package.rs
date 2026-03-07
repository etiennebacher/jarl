use biome_rowan::TextRange;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::fs::has_r_extension;
pub use crate::lints::base::duplicated_function_definition::duplicated_function_definition::is_in_r_package;
use crate::lints::base::duplicated_function_definition::duplicated_function_definition::{
    compute_duplicates_from_shared, scan_top_level_assignments,
};
use crate::lints::base::unused_function::unused_function::{
    collect_files, compute_unused_from_shared, has_cpp_extension, parse_namespace_s3_methods,
    scan_symbols,
};
use crate::rule_set::Rule;

/// Scope of a file within an R package, determining how its definitions
/// are checked for unused functions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileScope {
    /// R/ — definitions checked against all files; export check applies.
    R,
    /// tests/ — definitions checked only within tests/.
    Tests,
    /// inst/tinytest/ or inst/tests/ — definitions checked only within this scope.
    Inst,
    /// src/ — C/C++ files; no definition checking.
    Src,
}

/// Shared per-file data collected during the single parallel scan.
pub(crate) struct SharedFileData {
    pub root_key: String,
    pub rel_path: PathBuf,
    /// Absolute path to the package root (directory containing DESCRIPTION).
    pub package_root: PathBuf,
    pub assignments: Vec<(String, TextRange, u32, u32)>,
    pub symbol_counts: HashMap<String, usize>,
    pub scope: FileScope,
}

/// Pre-computed cross-file analysis results for an R package.
///
/// Separated from `Config` so that user settings and analysis results
/// live in different structs (following Ruff's pattern).
#[derive(Clone, Debug, Default)]
pub struct PackageAnalysis {
    /// Per-file duplicate top-level assignment data.
    /// Keyed by relativized file path. Value is a list of `(name, lhs_range,
    /// help)` triples where `help` points to the first definition.
    pub duplicate_assignments: HashMap<PathBuf, Vec<(String, TextRange, String)>>,
    /// Per-file unused internal function data.
    /// Keyed by relativized file path. Value is a list of `(name, lhs_range,
    /// help)` triples for functions that are defined but never called and not
    /// exported.
    pub unused_functions: HashMap<PathBuf, Vec<(String, TextRange, String)>>,
    /// S3 method names detected from NAMESPACE registrations and heuristic
    /// dot-prefix matching. Used by unused_function_argument to skip S3
    /// methods whose argument signatures are imposed by the generic.
    pub s3_methods: HashSet<String>,
}

/// Compute all package-level analysis for the given paths.
///
/// Performs a single parallel scan over all R-package files, reading each file
/// once and calling `scan_top_level_assignments` once (plus `scan_symbols` if
/// the unused-function rule is enabled). The results are then dispatched to
/// the duplicate and unused-function checkers.
pub fn compute_package_analysis(paths: &[PathBuf], config: &Config) -> PackageAnalysis {
    let rules = &config.rules_to_apply;
    let check_duplicates = rules.contains(&Rule::DuplicatedFunctionDefinition);
    let check_unused = rules.contains(&Rule::UnusedFunction);
    let check_unused_args = rules.contains(&Rule::UnusedFunctionArguments);
    let need_symbols = check_unused || check_unused_args;

    if !check_duplicates && !need_symbols {
        return PackageAnalysis::default();
    }

    // Cache is_in_r_package per unique parent directory so we do at most K
    // stat calls (typically 1) instead of N (one per file).
    let r_dirs: HashSet<PathBuf> = paths
        .iter()
        .filter(|p| has_r_extension(p))
        .filter_map(|p| p.parent().map(|d| d.to_path_buf()))
        .collect();

    let dir_is_package: HashMap<PathBuf, bool> = r_dirs
        .into_iter()
        .map(|dir| {
            let in_pkg = dir
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n == "R")
                && dir.parent().is_some_and(|p| p.join("DESCRIPTION").exists());
            (dir, in_pkg)
        })
        .collect();

    // Collect R/ files that belong to packages.
    let r_dir_files: Vec<&PathBuf> = paths
        .iter()
        .filter(|p| has_r_extension(p))
        .filter(|p| {
            p.parent()
                .and_then(|d| dir_is_package.get(d))
                .copied()
                .unwrap_or(false)
        })
        .collect();

    // Discover package roots and collect excluded R/ files so they still
    // contribute to cross-file analysis (both duplicate and unused checks).
    // Also collect extra files (tests/, inst/tinytest/, inst/tests/, src/) and
    // NAMESPACE exports for the unused-function rule.
    let mut extra_files: Vec<PathBuf> = Vec::new();
    let mut excluded_r_files: Vec<PathBuf> = Vec::new();
    let mut namespace_contents: HashMap<PathBuf, String> = HashMap::new();

    let package_roots: HashSet<PathBuf> = r_dir_files
        .iter()
        .filter_map(|p| p.parent().and_then(|r| r.parent()).map(|r| r.to_path_buf()))
        .collect();

    // Collect the set of R/ files already in paths (canonicalized for comparison).
    let r_dir_file_set: HashSet<PathBuf> = r_dir_files
        .iter()
        .filter_map(|p| std::fs::canonicalize(p).ok())
        .collect();

    for root in &package_roots {
        // Discover ALL R/ files on disk, including excluded ones, so they
        // contribute to the cross-file analysis. Diagnostics are only emitted
        // for files in config.paths, so excluded files won't produce warnings.
        let r_dir = root.join("R");
        if r_dir.is_dir() {
            for file in collect_files(&r_dir, has_r_extension) {
                if let Ok(canon) = std::fs::canonicalize(&file)
                    && !r_dir_file_set.contains(&canon)
                {
                    excluded_r_files.push(file);
                }
            }
        }

        if check_unused {
            // Collect test/tinytest R files
            for dir_name in &["inst/tinytest", "inst/tests", "tests"] {
                let dir = root.join(dir_name);
                if dir.is_dir() {
                    extra_files.extend(collect_files(&dir, has_r_extension));
                }
            }
            // Collect C/C++ files in src/
            let src_dir = root.join("src");
            if src_dir.is_dir() {
                extra_files.extend(collect_files(&src_dir, has_cpp_extension));
            }
        }
        if need_symbols {
            // Read NAMESPACE content (cheap, one per package)
            if let Ok(ns_content) = std::fs::read_to_string(root.join("NAMESPACE")) {
                namespace_contents.insert(root.clone(), ns_content);
            }
        }
    }

    // Build the list of all files to scan in parallel, each tagged with its scope.
    let all_files: Vec<(&Path, FileScope)> = r_dir_files
        .iter()
        .map(|p| (p.as_path(), FileScope::R))
        .chain(excluded_r_files.iter().map(|p| (p.as_path(), FileScope::R)))
        .chain(extra_files.iter().map(|p| {
            let scope = file_scope_from_path(p);
            (p.as_path(), scope)
        }))
        .collect();

    // Single parallel scan: read each file once. All R files get
    // scan_top_level_assignments; Src files only get scan_symbols.
    let shared_data: Vec<SharedFileData> = all_files
        .par_iter()
        .filter_map(|(path, scope)| {
            let content = std::fs::read_to_string(path).ok()?;
            let symbol_counts = if need_symbols {
                scan_symbols(&content)
            } else {
                HashMap::new()
            };

            let assignments = match scope {
                FileScope::Src => Vec::new(),
                _ => scan_top_level_assignments(&content),
            };

            if *scope == FileScope::R {
                let r_dir = path.parent()?;
                let package_root = r_dir.parent()?.to_path_buf();
                let rel_path = PathBuf::from(crate::fs::relativize_path(path));
                let root_key = crate::fs::relativize_path(r_dir);
                Some(SharedFileData {
                    root_key,
                    rel_path,
                    package_root,
                    assignments,
                    symbol_counts,
                    scope: FileScope::R,
                })
            } else {
                // Extra file: figure out the package root. The file is
                // somewhere under root/tests/, root/inst/, or root/src/.
                let package_root = find_package_root(path)?;
                let r_dir = package_root.join("R");
                let rel_path = PathBuf::from(crate::fs::relativize_path(path));
                let root_key = crate::fs::relativize_path(&r_dir);
                Some(SharedFileData {
                    root_key,
                    rel_path,
                    package_root,
                    assignments,
                    symbol_counts,
                    scope: *scope,
                })
            }
        })
        .collect();

    let duplicate_assignments = if check_duplicates {
        compute_duplicates_from_shared(&shared_data)
    } else {
        HashMap::new()
    };

    let unused_functions = if check_unused {
        compute_unused_from_shared(
            &shared_data,
            &config.rule_options.unused_function,
            &namespace_contents,
        )
    } else {
        HashMap::new()
    };

    let s3_methods = if check_unused_args {
        compute_s3_methods_from_shared(&shared_data, &namespace_contents)
    } else {
        HashSet::new()
    };

    PackageAnalysis {
        duplicate_assignments,
        unused_functions,
        s3_methods,
    }
}

/// Compute S3 method names from pre-scanned shared file data.
///
/// Combines two sources:
/// 1. Registered S3 methods from NAMESPACE `S3method()` directives.
/// 2. Probable S3 methods: function names containing a dot where a prefix
///    matches any symbol used in R/ files (e.g. `print.myclass` where
///    `print` appears as a symbol).
fn compute_s3_methods_from_shared(
    shared_data: &[SharedFileData],
    namespace_contents: &HashMap<PathBuf, String>,
) -> HashSet<String> {
    let mut all_s3: HashSet<String> = HashSet::new();

    // Group by package root
    let mut packages: HashMap<&str, Vec<&SharedFileData>> = HashMap::new();
    for fd in shared_data {
        packages.entry(&fd.root_key).or_default().push(fd);
    }

    for (_root_key, file_data) in packages {
        let r_files: Vec<&&SharedFileData> = file_data
            .iter()
            .filter(|f| f.scope == FileScope::R)
            .collect();

        let Some(first) = r_files.first() else {
            continue;
        };

        // 1. Registered S3 methods from NAMESPACE
        if let Some(ns_content) = namespace_contents.get(&first.package_root) {
            all_s3.extend(parse_namespace_s3_methods(ns_content));
        }

        // 2. Probable S3 methods via dot-prefix heuristic: if a function name
        //    contains a dot and a prefix before any dot matches a known symbol,
        //    it is likely an S3 method (e.g. `print.myclass`, `format.result`).
        let all_symbols: HashSet<&str> = r_files
            .iter()
            .flat_map(|f| f.symbol_counts.keys().map(|s| s.as_str()))
            .collect();

        for file in &r_files {
            for (name, _, _, _) in &file.assignments {
                if name.contains('.') {
                    let is_probable_s3 = name
                        .match_indices('.')
                        .any(|(pos, _)| all_symbols.contains(&name[..pos]));
                    if is_probable_s3 {
                        all_s3.insert(name.clone());
                    }
                }
            }
        }
    }

    all_s3
}

/// Determine the `FileScope` for a non-R/ file based on its path.
fn file_scope_from_path(path: &Path) -> FileScope {
    let components: Vec<_> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    for (i, comp) in components.iter().enumerate() {
        match comp.as_str() {
            "tests" => return FileScope::Tests,
            "inst" => {
                if let Some(next) = components.get(i + 1)
                    && (next == "tinytest" || next == "tests")
                {
                    return FileScope::Inst;
                }
            }
            "src" => return FileScope::Src,
            _ => {}
        }
    }
    // Fallback: treat unknown extra files as Tests scope
    FileScope::Tests
}

/// Walk up from a file path to find the package root (directory containing DESCRIPTION).
fn find_package_root(path: &Path) -> Option<PathBuf> {
    let mut dir = path.parent()?;
    loop {
        if dir.join("DESCRIPTION").exists() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
}

/// Scan paths into `SharedFileData`, reading each file once. Used by tests
/// that need to call `compute_duplicates_from_shared` /
/// `compute_unused_from_shared` directly.
#[cfg(test)]
pub(crate) fn scan_r_package_paths(paths: &[PathBuf], with_symbols: bool) -> Vec<SharedFileData> {
    paths
        .iter()
        .filter(|p| has_r_extension(p))
        .filter(|p| is_in_r_package(p).unwrap_or(false))
        .filter_map(|path| {
            let r_dir = path.parent()?;
            let package_root = r_dir.parent()?.to_path_buf();
            let rel_path = PathBuf::from(crate::fs::relativize_path(path));
            let root_key = crate::fs::relativize_path(r_dir);
            let content = std::fs::read_to_string(path).ok()?;
            let assignments = scan_top_level_assignments(&content);
            let symbol_counts = if with_symbols {
                scan_symbols(&content)
            } else {
                HashMap::new()
            };
            Some(SharedFileData {
                root_key,
                rel_path,
                package_root,
                assignments,
                symbol_counts,
                scope: FileScope::R,
            })
        })
        .collect()
}

/// Scan extra (non-R/) files into `SharedFileData` for tests. Assigns
/// the correct `FileScope` based on the file path and also collects
/// top-level assignments for R files.
/// The `package_root` is the directory containing DESCRIPTION.
#[cfg(test)]
pub(crate) fn scan_extra_package_paths(
    paths: &[PathBuf],
    package_root: &Path,
) -> Vec<SharedFileData> {
    let r_dir = package_root.join("R");
    let root_key = crate::fs::relativize_path(&r_dir);
    paths
        .iter()
        .filter_map(|path| {
            let content = std::fs::read_to_string(path).ok()?;
            let symbol_counts = scan_symbols(&content);
            let rel_path = PathBuf::from(crate::fs::relativize_path(path));
            let scope = file_scope_from_path(path);
            let assignments = match scope {
                FileScope::Src => Vec::new(),
                _ => scan_top_level_assignments(&content),
            };
            Some(SharedFileData {
                root_key: root_key.clone(),
                rel_path,
                package_root: package_root.to_path_buf(),
                assignments,
                symbol_counts,
                scope,
            })
        })
        .collect()
}
