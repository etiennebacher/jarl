use biome_rowan::TextRange;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::config::Config;
use crate::fs::has_r_extension;
pub use crate::lints::base::duplicated_function_definition::duplicated_function_definition::is_in_r_package;
use crate::lints::base::duplicated_function_definition::duplicated_function_definition::{
    compute_duplicates_from_shared, scan_top_level_assignments,
};
use crate::lints::base::unused_function::unused_function::{
    compute_unused_from_shared, scan_symbols,
};
use crate::rule_set::Rule;

/// Shared per-file data collected during the single parallel scan.
pub(crate) struct SharedFileData {
    pub root_key: String,
    pub rel_path: PathBuf,
    pub abs_path: PathBuf,
    pub assignments: Vec<(String, TextRange, u32, u32)>,
    pub symbol_counts: HashMap<String, usize>,
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

    // Single parallel scan: read each file once, call scan_top_level_assignments
    // once, and optionally scan_symbols.
    let shared_data: Vec<SharedFileData> = paths
        .par_iter()
        .filter(|p| has_r_extension(p))
        .filter(|p| {
            p.parent()
                .and_then(|d| dir_is_package.get(d))
                .copied()
                .unwrap_or(false)
        })
        .filter_map(|path| {
            let root = path.parent()?;
            let rel_path = PathBuf::from(crate::fs::relativize_path(path));
            let root_key = crate::fs::relativize_path(root);
            let content = std::fs::read_to_string(path).ok()?;
            let assignments = scan_top_level_assignments(&content);
            let symbol_counts = if check_unused {
                scan_symbols(&content)
            } else {
                HashMap::new()
            };
            Some(SharedFileData {
                root_key,
                rel_path,
                abs_path: path.clone(),
                assignments,
                symbol_counts,
            })
        })
        .collect();

    let duplicate_assignments = if check_duplicates {
        compute_duplicates_from_shared(&shared_data)
    } else {
        HashMap::new()
    };

    let unused_functions = if check_unused {
        compute_unused_from_shared(&shared_data, &config.rule_options.unused_function)
    } else {
        HashMap::new()
    };

    PackageAnalysis { duplicate_assignments, unused_functions }
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
            let root = path.parent()?;
            let rel_path = PathBuf::from(crate::fs::relativize_path(path));
            let root_key = crate::fs::relativize_path(root);
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
                abs_path: path.clone(),
                assignments,
                symbol_counts,
            })
        })
        .collect()
}
