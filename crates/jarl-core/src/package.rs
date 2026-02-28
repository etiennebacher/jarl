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
    collect_files, compute_unused_from_shared, has_cpp_extension, scan_symbols,
};
use crate::rule_set::Rule;

/// Shared per-file data collected during the single parallel scan.
pub(crate) struct SharedFileData {
    pub root_key: String,
    pub rel_path: PathBuf,
    /// Absolute path to the package root (directory containing DESCRIPTION).
    pub package_root: PathBuf,
    pub assignments: Vec<(String, TextRange, u32, u32)>,
    pub symbol_counts: HashMap<String, usize>,
    /// `true` for files in the R/ directory, `false` for extra files
    /// (tests/, inst/tinytest/, src/).
    pub is_r_dir_file: bool,
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

    if !check_duplicates && !check_unused {
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

    // Discover package roots and collect extra files (tests/, inst/tinytest/,
    // src/) for the unused-function rule. Also pre-compute NAMESPACE exports.
    let mut extra_files: Vec<PathBuf> = Vec::new();
    let mut namespace_contents: HashMap<PathBuf, String> = HashMap::new();

    if check_unused {
        let package_roots: HashSet<PathBuf> = r_dir_files
            .iter()
            .filter_map(|p| p.parent().and_then(|r| r.parent()).map(|r| r.to_path_buf()))
            .collect();

        for root in &package_roots {
            // Collect test/tinytest R files
            for dir_name in &["inst/tinytest", "tests"] {
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
            // Read NAMESPACE content (cheap, one per package)
            if let Ok(ns_content) = std::fs::read_to_string(root.join("NAMESPACE")) {
                namespace_contents.insert(root.clone(), ns_content);
            }
        }
    }

    // Build the list of all files to scan in parallel: R/ files (tagged as
    // is_r_dir_file=true) and extra files (tagged as is_r_dir_file=false).
    let all_files: Vec<(&Path, bool)> = r_dir_files
        .iter()
        .map(|p| (p.as_path(), true))
        .chain(extra_files.iter().map(|p| (p.as_path(), false)))
        .collect();

    // Single parallel scan: read each file once.
    // - R/ files: scan_top_level_assignments + optionally scan_symbols.
    // - Extra files: only scan_symbols (no assignments needed).
    let shared_data: Vec<SharedFileData> = all_files
        .par_iter()
        .filter_map(|(path, is_r_dir)| {
            let content = std::fs::read_to_string(path).ok()?;
            let symbol_counts = if check_unused {
                scan_symbols(&content)
            } else {
                HashMap::new()
            };

            if *is_r_dir {
                let r_dir = path.parent()?;
                let package_root = r_dir.parent()?.to_path_buf();
                let rel_path = PathBuf::from(crate::fs::relativize_path(path));
                let root_key = crate::fs::relativize_path(r_dir);
                let assignments = scan_top_level_assignments(&content);
                Some(SharedFileData {
                    root_key,
                    rel_path,

                    package_root,
                    assignments,
                    symbol_counts,
                    is_r_dir_file: true,
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
                    assignments: Vec::new(),
                    symbol_counts,
                    is_r_dir_file: false,
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

    PackageAnalysis { duplicate_assignments, unused_functions }
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
                is_r_dir_file: true,
            })
        })
        .collect()
}

/// Scan extra (non-R/) files into `SharedFileData` for tests. These are
/// test/tinytest/src files that should have `is_r_dir_file = false`.
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
            Some(SharedFileData {
                root_key: root_key.clone(),
                rel_path,

                package_root: package_root.to_path_buf(),
                assignments: Vec::new(),
                symbol_counts,
                is_r_dir_file: false,
            })
        })
        .collect()
}
