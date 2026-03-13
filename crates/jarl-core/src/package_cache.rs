//! Lazy cache of installed R package metadata.
//!
//! Uses a single `Rscript` call to batch-query exports, versions, and install
//! paths for the packages we care about. Mtime-based staleness checks avoid
//! re-running Rscript unless a package actually changed on disk.
//!
//! Caches are keyed by R project root so that files in an renv project get
//! exports from that project's library, not the system library.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use crate::checker::PackageOrigin;

/// Information about an installed R package.
#[derive(Debug, Clone)]
pub struct PackageInfo {
    /// Exported function/object names.
    pub exports: HashSet<String>,
    /// Package version from DESCRIPTION (e.g., `(1, 2, 0)`).
    pub version: Option<(u32, u32, u32)>,
    /// Install path on disk (e.g. `/home/user/R/x86_64-pc-linux-gnu-library/4.5`).
    /// Used for mtime-based staleness checks.
    pub install_path: Option<PathBuf>,
}

/// Cache of installed R package metadata for a single R environment.
///
/// Each R project root (renv or system) gets its own `PackageCache`.
#[derive(Debug)]
pub struct PackageCache {
    /// Package name → info (`None` means looked up but not found).
    cache: RwLock<HashMap<String, Option<PackageInfo>>>,
    /// Modification times of package DESCRIPTION files at the time of lookup,
    /// used for staleness detection.
    mtimes: RwLock<HashMap<String, Option<SystemTime>>>,
    /// The project root this cache was created for, used as the working
    /// directory when re-running Rscript for staleness refreshes.
    project_root: Option<PathBuf>,
}

impl PackageCache {
    /// Query R for package metadata and build a cache.
    ///
    /// Spawns a single `Rscript` process with `project_root` as its working
    /// directory (so renv auto-activates via `.Rprofile`). Packages that are
    /// not installed are silently skipped.
    pub fn from_rscript(packages: &[&str], project_root: Option<&Path>) -> Option<Self> {
        if packages.is_empty() {
            return None;
        }

        let result = run_rscript_for_pkg_info(packages, project_root)?;
        if result.cache.is_empty() {
            return None;
        }

        Some(Self {
            cache: RwLock::new(result.cache),
            mtimes: RwLock::new(result.mtimes),
            project_root: project_root.map(Path::to_path_buf),
        })
    }

    /// Build an in-memory cache from a list of (package_name, exports) pairs.
    ///
    /// No filesystem access is performed. Useful for testing.
    pub fn from_exports(entries: &[(&str, &[&str])]) -> Self {
        let mut cache = HashMap::new();
        for (pkg_name, exports) in entries {
            let info = PackageInfo {
                exports: exports.iter().map(|s| s.to_string()).collect(),
                version: None,
                install_path: None,
            };
            cache.insert(pkg_name.to_string(), Some(info));
        }
        Self {
            cache: RwLock::new(cache),
            mtimes: RwLock::new(HashMap::new()),
            project_root: None,
        }
    }

    /// Look up a package from the cache.
    pub fn get(&self, name: &str) -> Option<PackageInfo> {
        let cache = self.cache.read().unwrap();
        cache.get(name).and_then(|v| v.clone())
    }

    /// Check if any packages were loaded into the cache.
    pub fn is_available(&self) -> bool {
        let cache = self.cache.read().unwrap();
        !cache.is_empty()
    }

    /// Check whether any of the given packages have changed on disk since
    /// they were cached (by comparing DESCRIPTION mtime). Stale entries are
    /// re-fetched via Rscript.
    ///
    /// Returns the names of packages that were refreshed.
    pub fn refresh_if_stale(&self, packages: &[&str]) -> Vec<String> {
        let mut stale = Vec::new();
        let mtimes = self.mtimes.read().unwrap();

        for &pkg in packages {
            let Some(recorded) = mtimes.get(pkg) else {
                continue;
            };
            let current = self.description_mtime(pkg);
            if current != *recorded {
                stale.push(pkg.to_string());
            }
        }
        drop(mtimes);

        if stale.is_empty() {
            return Vec::new();
        }

        // Re-fetch stale packages via Rscript
        let stale_refs: Vec<&str> = stale.iter().map(|s| s.as_str()).collect();
        if let Some(result) = run_rscript_for_pkg_info(&stale_refs, self.project_root.as_deref()) {
            let mut cache = self.cache.write().unwrap();
            let mut mtimes = self.mtimes.write().unwrap();
            for pkg in &stale {
                if let Some(info) = result.cache.get(pkg) {
                    cache.insert(pkg.clone(), info.clone());
                } else {
                    cache.insert(pkg.clone(), None);
                }
                if let Some(mtime) = result.mtimes.get(pkg) {
                    mtimes.insert(pkg.clone(), *mtime);
                } else {
                    mtimes.remove(pkg);
                }
            }
        } else {
            // Rscript failed — evict stale entries so they don't stay stale forever
            let mut cache = self.cache.write().unwrap();
            let mut mtimes = self.mtimes.write().unwrap();
            for pkg in &stale {
                cache.remove(pkg);
                mtimes.remove(pkg);
            }
        }

        stale
    }

    /// Get the mtime of a package's DESCRIPTION file at its recorded install path.
    fn description_mtime(&self, name: &str) -> Option<SystemTime> {
        let cache = self.cache.read().unwrap();
        let info = cache.get(name)?.as_ref()?;
        let install_path = info.install_path.as_ref()?;
        let desc = install_path.join(name).join("DESCRIPTION");
        std::fs::metadata(&desc).ok()?.modified().ok()
    }
}

/// Map of R project roots to their per-environment package caches.
///
/// Different projects may have different R library paths (e.g. renv vs system).
/// This map ensures that each project gets its own `PackageCache` while
/// minimizing the number of Rscript calls (one per unique project root).
///
/// Directory-to-root lookups are cached so that `find_r_project_root` only
/// walks the filesystem once per parent directory, not once per file.
#[derive(Debug, Default)]
pub struct PackageCacheMap {
    caches: RwLock<HashMap<Option<PathBuf>, Arc<PackageCache>>>,
    /// Cache of parent directory → resolved project root. Avoids repeated
    /// `stat()` walks up the directory tree for files in the same directory.
    root_cache: RwLock<HashMap<PathBuf, Option<PathBuf>>>,
}

impl PackageCacheMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve the R project root for a file, using the directory-level cache.
    fn resolve_root(&self, file_path: &Path) -> Option<PathBuf> {
        let dir = if file_path.is_file() {
            file_path.parent().unwrap_or(file_path)
        } else {
            file_path
        };

        // Fast path: directory already resolved
        {
            let cache = self.root_cache.read().unwrap();
            if let Some(root) = cache.get(dir) {
                return root.clone();
            }
        }

        // Slow path: walk the filesystem
        let root = find_r_project_root(file_path);

        let mut cache = self.root_cache.write().unwrap();
        cache.insert(dir.to_path_buf(), root.clone());
        root
    }

    /// Get or create a `PackageCache` for the given file path.
    ///
    /// Resolves the file's R project root (renv or workspace root), then
    /// either returns the existing cache for that root or creates a new one
    /// via `Rscript`.
    pub fn get_or_create(&self, file_path: &Path, packages: &[&str]) -> Option<Arc<PackageCache>> {
        let root = self.resolve_root(file_path);

        // Fast path: already cached for this root
        {
            let caches = self.caches.read().unwrap();
            if let Some(cache) = caches.get(&root) {
                return Some(Arc::clone(cache));
            }
        }

        // Slow path: create a new cache
        let cache = PackageCache::from_rscript(packages, root.as_deref())?;
        let cache = Arc::new(cache);

        let mut caches = self.caches.write().unwrap();
        // Another thread may have raced us; use the existing entry if so.
        caches.entry(root).or_insert_with(|| Arc::clone(&cache));
        Some(cache)
    }

    /// Get the existing cache for a file's project root, if any.
    pub fn get_for_file(&self, file_path: &Path) -> Option<Arc<PackageCache>> {
        let root = self.resolve_root(file_path);
        let caches = self.caches.read().unwrap();
        caches.get(&root).cloned()
    }
}

/// Find the R project root for a given file path.
///
/// Walks up the directory tree looking for markers of an R project environment:
/// - `renv.lock` — renv project (changes `.libPaths()` via auto-activation)
/// - `DESCRIPTION` — R package root
///
/// Returns `None` if no marker is found (system R installation).
pub fn find_r_project_root(file_path: &Path) -> Option<PathBuf> {
    let start = if file_path.is_file() {
        file_path.parent()?
    } else {
        file_path
    };

    let mut dir = start;
    loop {
        // renv takes priority: it changes the entire library path
        if dir.join("renv.lock").exists() {
            return Some(dir.to_path_buf());
        }
        if dir.join("DESCRIPTION").exists() {
            return Some(dir.to_path_buf());
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => return None,
        }
    }
}

/// Per-file package context for resolving bare function names to packages.
///
/// Built during the pre-pass from `library()`/`require()` calls found in
/// the file. Holds a reference to the shared `PackageCache`.
pub struct FilePackageContext<'a> {
    /// Packages loaded in this file, in load order.
    loaded_packages: Vec<String>,
    /// Reference to the shared cache.
    cache: &'a PackageCache,
}

impl<'a> FilePackageContext<'a> {
    pub fn new(loaded_packages: Vec<String>, cache: &'a PackageCache) -> Self {
        Self { loaded_packages, cache }
    }

    /// Resolve which package a bare function name comes from.
    pub fn resolve_package(&self, fn_name: &str) -> PackageOrigin {
        let mut candidates: Vec<String> = Vec::new();
        for pkg_name in &self.loaded_packages {
            if let Some(info) = self.cache.get(pkg_name)
                && info.exports.contains(fn_name)
            {
                candidates.push(pkg_name.clone());
            }
        }

        match candidates.len() {
            0 => PackageOrigin::Unknown,
            1 => PackageOrigin::Resolved(candidates.into_iter().next().unwrap()),
            _ => PackageOrigin::Ambiguous(candidates),
        }
    }

    /// Look up version info for a specific package.
    pub fn package_version(&self, pkg_name: &str) -> Option<(u32, u32, u32)> {
        self.cache.get(pkg_name).and_then(|info| info.version)
    }
}

/// Result of a batch Rscript query for package metadata.
struct PackageBatchResult {
    /// Link package name to info (`None` means looked up but not found).
    cache: HashMap<String, Option<PackageInfo>>,
    /// Link package name to DESCRIPTION mtime at query time, for staleness detection
    /// in LSP.
    mtimes: HashMap<String, Option<SystemTime>>,
}

/// Run a single Rscript process that returns exports, version, and install
/// path for each requested package.
///
/// When `project_root` is set, the process runs with that directory as its
/// working directory so that renv auto-activates via `.Rprofile`.
///
/// Returns `None` if Rscript failed entirely.
fn run_rscript_for_pkg_info(
    packages: &[&str],
    project_root: Option<&Path>,
) -> Option<PackageBatchResult> {
    let pkg_vec: String = packages
        .iter()
        .map(|p| format!("\"{}\"", p.replace('\\', "\\\\").replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(", ");

    let script = format!(
        r#"for (pkg in c({pkg_vec})) {{
  tryCatch({{
    cat(pkg, "\n", sep = "")
    cat(format(packageVersion(pkg)), "\n", sep = "")
    cat(dirname(system.file(package = pkg)), "\n", sep = "")
    cat(paste(getNamespaceExports(pkg), collapse = "\n"), "\n", sep = "")
    cat("---\n")
  }}, error = function(e) NULL)
}}"#
    );

    let mut cmd = Command::new("Rscript");
    cmd.args(["-e", &script]);

    if let Some(root) = project_root
        && root.is_dir()
    {
        cmd.current_dir(root);
    }

    let output = cmd.output().ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let mut cache_map: HashMap<String, Option<PackageInfo>> = HashMap::new();
    let mut mtime_map: HashMap<String, Option<SystemTime>> = HashMap::new();

    for block in stdout.split("---\n") {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }

        let mut lines = block.lines();
        let Some(name) = lines.next() else { continue };
        let name = name.trim().to_string();
        let Some(version_str) = lines.next().map(str::trim) else {
            continue;
        };
        let Some(install_path_str) = lines.next().map(str::trim) else {
            continue;
        };
        let exports: HashSet<String> = lines
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();

        let version = parse_package_version(version_str);
        let install_path = PathBuf::from(install_path_str);

        // Record DESCRIPTION mtime for staleness detection
        let desc_mtime = install_path
            .join(&name)
            .join("DESCRIPTION")
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok());
        mtime_map.insert(name.clone(), desc_mtime);

        let info = PackageInfo { exports, version, install_path: Some(install_path) };
        cache_map.insert(name, Some(info));
    }

    Some(PackageBatchResult { cache: cache_map, mtimes: mtime_map })
}

/// Parse a version string like "1.2.3" into a tuple.
fn parse_package_version(version: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let major = parts[0].parse().ok()?;
    let minor = parts[1].parse().ok()?;
    let patch = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_exports() {
        let cache = PackageCache::from_exports(&[
            ("dplyr", &["filter", "mutate", "select"]),
            ("tidyr", &["pivot_longer"]),
        ]);

        let info = cache.get("dplyr").unwrap();
        assert!(info.exports.contains("filter"));
        assert!(info.exports.contains("mutate"));
        assert!(info.exports.contains("select"));

        let info = cache.get("tidyr").unwrap();
        assert!(info.exports.contains("pivot_longer"));

        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_file_package_context_resolve() {
        let cache = PackageCache::from_exports(&[
            ("dplyr", &["filter", "mutate"]),
            ("stats", &["filter", "lag"]),
        ]);

        let ctx = FilePackageContext::new(vec!["stats".to_string(), "dplyr".to_string()], &cache);
        assert_eq!(
            ctx.resolve_package("filter"),
            PackageOrigin::Ambiguous(vec!["stats".to_string(), "dplyr".to_string()])
        );
        assert_eq!(
            ctx.resolve_package("lag"),
            PackageOrigin::Resolved("stats".to_string())
        );
        assert_eq!(ctx.resolve_package("nonexistent"), PackageOrigin::Unknown);
    }

    #[test]
    fn test_parse_package_version() {
        assert_eq!(parse_package_version("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_package_version("1.2"), Some((1, 2, 0)));
        assert_eq!(parse_package_version("0.10.1"), Some((0, 10, 1)));
        assert_eq!(parse_package_version("1"), None);
    }

    #[test]
    fn test_run_rscript_for_pkg_info_integration() {
        // Skip if R is not available
        if Command::new("Rscript")
            .args(["-e", "cat('ok')"])
            .output()
            .is_err()
        {
            return;
        }

        let cache = PackageCache::from_rscript(&["base"], None);
        // base is always available
        if let Some(cache) = cache {
            let info = cache.get("base").unwrap();
            assert!(info.exports.contains("cat"));
            assert!(info.exports.contains("print"));
            assert!(info.install_path.is_some());
        }
    }

    #[test]
    fn test_export_pattern_via_rscript() {
        // Test against the real `astsa` package which uses:
        //   exportPattern("^[^\\.]")
        // This exports all objects not starting with a dot.
        // Skip if R or astsa is not available.
        let ok = Command::new("Rscript")
            .args(["-e", "library(astsa)"])
            .output()
            .is_ok_and(|o| o.status.success());
        if !ok {
            return;
        }

        let cache = PackageCache::from_rscript(&["astsa"], None).unwrap();
        let info = cache.get("astsa").unwrap();

        // Ground truth from R: sort(getNamespaceExports("astsa"))
        let mut expected = vec![
            "%^%",
            "acf1",
            "acf2",
            "acfm",
            "ar.boot",
            "ar.mcmc",
            "arma.check",
            "arma.spec",
            "ARMAtoAR",
            "astsa.col",
            "autoParm",
            "autoSpec",
            "bart",
            "ccf2",
            "detrend",
            "dna2vector",
            "EM",
            "ESS",
            "FDR",
            "ffbs",
            "Grid",
            "Kfilter",
            "Ksmooth",
            "lag1.plot",
            "lag2.plot",
            "LagReg",
            "matrixpwr",
            "mvspec",
            "polyMul",
            "pre.white",
            "QQnorm",
            "sarima",
            "sarima.for",
            "sarima.sim",
            "scatter.hist",
            "SigExtract",
            "spec.ic",
            "specenv",
            "ssm",
            "stoch.reg",
            "SV.mcmc",
            "SV.mle",
            "test.linear",
            "timex",
            "trend",
            "tspairs",
            "tsplot",
            "ttable",
        ];
        expected.sort();

        let mut actual: Vec<&str> = info.exports.iter().map(|s| s.as_str()).collect();
        actual.sort();

        assert_eq!(actual, expected);
    }

    // ── find_r_project_root tests ──────────────────────────────────────

    #[test]
    fn test_find_root_no_markers() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("script.R");
        std::fs::write(&file, "").unwrap();

        assert_eq!(find_r_project_root(&file), None);
    }

    #[test]
    fn test_find_root_description_in_parent() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("DESCRIPTION"),
            "Package: mypkg\nVersion: 0.1.0\n",
        )
        .unwrap();
        let r_dir = dir.path().join("R");
        std::fs::create_dir(&r_dir).unwrap();
        let file = r_dir.join("foo.R");
        std::fs::write(&file, "").unwrap();

        assert_eq!(find_r_project_root(&file), Some(dir.path().to_path_buf()));
    }

    #[test]
    fn test_find_root_renv_takes_priority_over_description() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("DESCRIPTION"),
            "Package: mypkg\nVersion: 0.1.0\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("renv.lock"), "{}").unwrap();
        let file = dir.path().join("R").join("foo.R");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "").unwrap();

        // renv.lock is found at the same level — that's the root
        assert_eq!(find_r_project_root(&file), Some(dir.path().to_path_buf()));
    }

    #[test]
    fn test_find_root_nested_renv_inside_workspace() {
        // workspace/
        //   renv.lock
        //   subproject/
        //     DESCRIPTION
        //     R/foo.R
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("renv.lock"), "{}").unwrap();

        let sub = dir.path().join("subproject");
        std::fs::create_dir_all(sub.join("R")).unwrap();
        std::fs::write(sub.join("DESCRIPTION"), "Package: subpkg\nVersion: 0.1.0\n").unwrap();
        let file = sub.join("R").join("foo.R");
        std::fs::write(&file, "").unwrap();

        // DESCRIPTION is closer, so the subproject is the root
        assert_eq!(find_r_project_root(&file), Some(sub));
    }

    #[test]
    fn test_find_root_file_at_root_level() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("DESCRIPTION"),
            "Package: mypkg\nVersion: 0.1.0\n",
        )
        .unwrap();
        let file = dir.path().join("script.R");
        std::fs::write(&file, "").unwrap();

        assert_eq!(find_r_project_root(&file), Some(dir.path().to_path_buf()));
    }

    // ── PackageCacheMap tests ──────────────────────────────────────────

    #[test]
    fn test_cache_map_returns_same_cache_for_same_root() {
        let map = PackageCacheMap::new();

        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("DESCRIPTION"),
            "Package: mypkg\nVersion: 0.1.0\n",
        )
        .unwrap();

        let r_dir = dir.path().join("R");
        std::fs::create_dir(&r_dir).unwrap();
        let file_a = r_dir.join("a.R");
        let file_b = r_dir.join("b.R");
        std::fs::write(&file_a, "").unwrap();
        std::fs::write(&file_b, "").unwrap();

        // Skip if R is not available
        if Command::new("Rscript")
            .args(["-e", "cat('ok')"])
            .output()
            .is_err()
        {
            return;
        }

        let cache_a = map.get_or_create(&file_a, &["base"]);
        let cache_b = map.get_or_create(&file_b, &["base"]);

        // Both files share the same project root, so they should get the
        // same Arc (pointer equality).
        assert!(cache_a.is_some());
        assert!(Arc::ptr_eq(
            cache_a.as_ref().unwrap(),
            cache_b.as_ref().unwrap()
        ));
    }

    #[test]
    fn test_cache_map_different_roots_get_separate_caches() {
        let map = PackageCacheMap::new();

        // Two separate project directories
        let dir_a = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir_a.path().join("DESCRIPTION"),
            "Package: pkg_a\nVersion: 0.1.0\n",
        )
        .unwrap();
        let file_a = dir_a.path().join("script.R");
        std::fs::write(&file_a, "").unwrap();

        let dir_b = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir_b.path().join("DESCRIPTION"),
            "Package: pkg_b\nVersion: 0.2.0\n",
        )
        .unwrap();
        let file_b = dir_b.path().join("script.R");
        std::fs::write(&file_b, "").unwrap();

        // Skip if R is not available
        if Command::new("Rscript")
            .args(["-e", "cat('ok')"])
            .output()
            .is_err()
        {
            return;
        }

        let cache_a = map.get_or_create(&file_a, &["base"]);
        let cache_b = map.get_or_create(&file_b, &["base"]);

        // Different project roots → different cache instances
        assert!(cache_a.is_some());
        assert!(cache_b.is_some());
        assert!(!Arc::ptr_eq(
            cache_a.as_ref().unwrap(),
            cache_b.as_ref().unwrap()
        ));
    }

    #[test]
    fn test_cache_map_no_root_files_share_system_cache() {
        let map = PackageCacheMap::new();

        // Files outside any R project → root is None → share one cache
        let dir = tempfile::TempDir::new().unwrap();
        let file_a = dir.path().join("a.R");
        let file_b = dir.path().join("b.R");
        std::fs::write(&file_a, "").unwrap();
        std::fs::write(&file_b, "").unwrap();

        // Skip if R is not available
        if Command::new("Rscript")
            .args(["-e", "cat('ok')"])
            .output()
            .is_err()
        {
            return;
        }

        let cache_a = map.get_or_create(&file_a, &["base"]);
        let cache_b = map.get_or_create(&file_b, &["base"]);

        assert!(cache_a.is_some());
        assert!(Arc::ptr_eq(
            cache_a.as_ref().unwrap(),
            cache_b.as_ref().unwrap()
        ));
    }

    #[test]
    fn test_cache_map_nested_dirs_same_root() {
        let map = PackageCacheMap::new();

        // project/
        //   DESCRIPTION
        //   R/foo.R
        //   R/sub/bar.R   (nested)
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("DESCRIPTION"),
            "Package: mypkg\nVersion: 0.1.0\n",
        )
        .unwrap();
        let r_dir = dir.path().join("R");
        std::fs::create_dir(&r_dir).unwrap();
        let sub_dir = r_dir.join("sub");
        std::fs::create_dir(&sub_dir).unwrap();

        let file_a = r_dir.join("foo.R");
        let file_b = sub_dir.join("bar.R");
        std::fs::write(&file_a, "").unwrap();
        std::fs::write(&file_b, "").unwrap();

        // Skip if R is not available
        if Command::new("Rscript")
            .args(["-e", "cat('ok')"])
            .output()
            .is_err()
        {
            return;
        }

        let cache_a = map.get_or_create(&file_a, &["base"]);
        let cache_b = map.get_or_create(&file_b, &["base"]);

        // Same project root → same cache
        assert!(cache_a.is_some());
        assert!(Arc::ptr_eq(
            cache_a.as_ref().unwrap(),
            cache_b.as_ref().unwrap()
        ));
    }

    #[test]
    fn test_cache_map_renv_project_vs_plain() {
        let map = PackageCacheMap::new();

        // renv project
        let renv_dir = tempfile::TempDir::new().unwrap();
        std::fs::write(renv_dir.path().join("renv.lock"), "{}").unwrap();
        let renv_file = renv_dir.path().join("analysis.R");
        std::fs::write(&renv_file, "").unwrap();

        // plain project
        let plain_dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            plain_dir.path().join("DESCRIPTION"),
            "Package: plainpkg\nVersion: 0.1.0\n",
        )
        .unwrap();
        let plain_file = plain_dir.path().join("script.R");
        std::fs::write(&plain_file, "").unwrap();

        // Skip if R is not available
        if Command::new("Rscript")
            .args(["-e", "cat('ok')"])
            .output()
            .is_err()
        {
            return;
        }

        let cache_renv = map.get_or_create(&renv_file, &["base"]);
        let cache_plain = map.get_or_create(&plain_file, &["base"]);

        // Different roots → different caches
        assert!(cache_renv.is_some());
        assert!(cache_plain.is_some());
        assert!(!Arc::ptr_eq(
            cache_renv.as_ref().unwrap(),
            cache_plain.as_ref().unwrap()
        ));
    }
}
