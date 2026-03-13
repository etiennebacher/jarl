//! Lazy cache of installed R package metadata.
//!
//! Uses a single `Rscript` call to batch-query exports, versions, and install
//! paths for the packages we care about. Mtime-based staleness checks avoid
//! re-running Rscript unless a package actually changed on disk.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use std::sync::RwLock;
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

/// Lazily-populated cache of installed R package metadata.
///
/// Shared across all files being linted (wrapped in `Arc` for thread safety).
#[derive(Debug)]
pub struct PackageCache {
    /// Lazily populated: package name → info (`None` means looked up but not found).
    cache: RwLock<HashMap<String, Option<PackageInfo>>>,
    /// Modification times of package DESCRIPTION files at the time of lookup,
    /// used for staleness detection.
    mtimes: RwLock<HashMap<String, Option<SystemTime>>>,
}

impl PackageCache {
    /// Query R for package metadata and build a cache.
    ///
    /// Spawns a single `Rscript` process that returns exports, version, and
    /// install path for each requested package. Packages that are not installed
    /// are silently skipped.
    pub fn from_rscript(packages: &[&str]) -> Option<Self> {
        if packages.is_empty() {
            return None;
        }

        let result = run_rscript_for_pkg_info(packages)?;
        if result.cache.is_empty() {
            return None;
        }

        Some(Self {
            cache: RwLock::new(result.cache),
            mtimes: RwLock::new(result.mtimes),
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
        if let Some(result) = run_rscript_for_pkg_info(&stale_refs) {
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
/// Returns `None` if Rscript failed entirely.
fn run_rscript_for_pkg_info(packages: &[&str]) -> Option<PackageBatchResult> {
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

    let output = Command::new("Rscript")
        .args(["-e", &script])
        .output()
        .ok()?;

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

        let cache = PackageCache::from_rscript(&["base"]);
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

        let cache = PackageCache::from_rscript(&["astsa"]).unwrap();
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
}
