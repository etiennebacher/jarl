use biome_rowan::TextRange;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::checker::DEFAULT_PACKAGES;
use crate::config::Config;
use crate::description::Description;
use crate::fs::has_r_extension;
pub use crate::lints::base::duplicated_function_definition::duplicated_function_definition::is_in_r_package;
use crate::lints::base::duplicated_function_definition::duplicated_function_definition::{
    compute_duplicates_from_shared, scan_top_level_assignments,
};
use crate::lints::base::unused_function::unused_function::{
    collect_files, compute_unused_from_shared, has_cpp_extension, scan_symbols,
};
use crate::namespace::{
    S3Info, parse_namespace_exports, parse_namespace_imports, parse_namespace_s3,
};
use crate::rule_set::Rule;

/// Scope of a file within an R package, determining how its definitions
/// are checked for unused functions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileScope {
    /// R/ — definitions checked against all files; export check applies.
    R,
    /// tests/ — definitions checked only within tests/.
    Tests,
    /// inst/tinytest/ or inst/tests/ — definitions checked only within this scope.
    Inst,
    /// src/ — C/C++ files; no definition checking.
    Src,
}

/// Pre-computed package metadata from DESCRIPTION + NAMESPACE.
/// One instance per package root.
#[derive(Clone, Debug, Default)]
pub struct PackageContext {
    pub namespace_exports: HashSet<String>,
    pub import_from: HashMap<String, String>,
    pub loaded_packages: Vec<String>,
    /// Raw NAMESPACE content, retained so `compute_unused_from_shared()` can
    /// call `parse_namespace_exports()` with the full `all_names` list.
    pub namespace_content: Option<String>,
    /// S3 generic and method names extracted from `S3method()` directives.
    pub s3_info: S3Info,
}

/// Per-file package classification, computed upfront by
/// `summarize_package_info()`.
#[derive(Clone, Debug)]
pub enum FilePackageInfo {
    InPackage {
        package_root: PathBuf,
        scope: FileScope,
    },
    Script,
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
}

/// Classify every file and pre-compute per-package metadata in one pass.
///
/// For each R-package file the function identifies its package root and scope.
/// For each unique package root it reads DESCRIPTION and NAMESPACE once,
/// building a [`PackageContext`] that downstream code can use without
/// touching the filesystem again.
pub fn summarize_package_info(
    paths: &[PathBuf],
) -> (
    HashMap<PathBuf, PackageContext>,
    HashMap<PathBuf, FilePackageInfo>,
) {
    // Cache is_in_r_package per unique parent directory.
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

    let mut file_info: HashMap<PathBuf, FilePackageInfo> = HashMap::new();
    let mut package_roots: HashSet<PathBuf> = HashSet::new();

    // Insert file info under both the original path and its relativized form,
    // since downstream code may look up by either.
    let mut insert_info = |path: &PathBuf, info: FilePackageInfo| {
        let rel = PathBuf::from(crate::fs::relativize_path(path));
        file_info.insert(path.clone(), info.clone());
        if rel != *path {
            file_info.insert(rel, info);
        }
    };

    for path in paths {
        if !has_r_extension(path) {
            insert_info(path, FilePackageInfo::Script);
            continue;
        }

        // Check if this file is in an R/ directory inside a package.
        let in_pkg = path
            .parent()
            .and_then(|d| dir_is_package.get(d))
            .copied()
            .unwrap_or(false);

        if in_pkg && let Some(pkg_root) = path.parent().and_then(|r| r.parent()) {
            let pkg_root = pkg_root.to_path_buf();
            package_roots.insert(pkg_root.clone());
            insert_info(
                path,
                FilePackageInfo::InPackage { package_root: pkg_root, scope: FileScope::R },
            );
            continue;
        }

        // Not in R/ — check if the file is under a recognized package
        // subdirectory (tests/, inst/tinytest, inst/tests, src/).
        // Files in other directories (data-raw/, vignettes/, etc.) are
        // treated as scripts so that their `library()` calls are scanned.
        if let Some(pkg_root) = find_package_root(path) {
            let scope = file_scope_from_path(path);
            if is_known_package_scope(path, &pkg_root) {
                package_roots.insert(pkg_root.clone());
                insert_info(
                    path,
                    FilePackageInfo::InPackage { package_root: pkg_root, scope },
                );
            } else {
                insert_info(path, FilePackageInfo::Script);
            }
        } else {
            insert_info(path, FilePackageInfo::Script);
        }
    }

    // Build a PackageContext for each unique package root.
    let mut contexts: HashMap<PathBuf, PackageContext> = HashMap::new();
    for root in &package_roots {
        let mut packages: Vec<String> = DEFAULT_PACKAGES.iter().map(|s| s.to_string()).collect();
        let mut import_from = HashMap::new();
        let mut namespace_exports = HashSet::new();
        let mut namespace_content = None;

        let desc_path = root.join("DESCRIPTION");
        if let Ok(desc) = std::fs::read_to_string(&desc_path) {
            packages.extend(Description::get_package_deps(
                &desc,
                &["Depends", "Imports"],
            ));
        }

        let mut s3_info = S3Info::default();
        let ns_path = root.join("NAMESPACE");
        if let Ok(ns) = std::fs::read_to_string(&ns_path) {
            let imports = parse_namespace_imports(&ns);
            import_from = imports.import_from;
            for pkg in imports.blanket_imports {
                if !packages.contains(&pkg) {
                    packages.push(pkg);
                }
            }
            namespace_exports = parse_namespace_exports(&ns, &[]);
            s3_info = parse_namespace_s3(&ns);
            namespace_content = Some(ns);
        }

        contexts.insert(
            root.clone(),
            PackageContext {
                namespace_exports,
                import_from,
                loaded_packages: packages,
                namespace_content,
                s3_info,
            },
        );
    }

    (contexts, file_info)
}

/// Compute all package-level analysis for the given paths.
///
/// Performs a single parallel scan over all R-package files, reading each file
/// once and calling `scan_top_level_assignments` once (plus `scan_symbols` if
/// the unused-function rule is enabled). The results are then dispatched to
/// the duplicate and unused-function checkers.
pub fn make_package_analysis(
    paths: &[PathBuf],
    config: &Config,
    namespace_contents: &HashMap<PathBuf, String>,
) -> PackageAnalysis {
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

    // Discover package roots and collect excluded R/ files so they still
    // contribute to cross-file analysis (both duplicate and unused checks).
    // Also collect extra files (tests/, inst/tinytest/, inst/tests/, src/).
    let mut extra_files: Vec<PathBuf> = Vec::new();
    let mut excluded_r_files: Vec<PathBuf> = Vec::new();

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
            let symbol_counts = if check_unused {
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
            namespace_contents,
        )
    } else {
        HashMap::new()
    };

    PackageAnalysis { duplicate_assignments, unused_functions }
}

/// Determine the `FileScope` for a non-R/ file based on its path.
pub(crate) fn file_scope_from_path(path: &Path) -> FileScope {
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

/// Check whether a file is under a recognized package subdirectory
/// (tests/, inst/tinytest, inst/tests, src/) relative to the package root.
fn is_known_package_scope(path: &Path, package_root: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(package_root) else {
        return false;
    };
    let first = rel.components().next().and_then(|c| c.as_os_str().to_str());
    matches!(first, Some("R" | "tests" | "src" | "inst"))
}

/// Walk up from a file path to find the package root (directory containing DESCRIPTION).
pub(crate) fn find_package_root(path: &Path) -> Option<PathBuf> {
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
