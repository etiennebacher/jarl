//! Lazy cache of installed R package metadata.
//!
//! Looks up package NAMESPACE (exports) and DESCRIPTION (version) on demand
//! from the R library paths discovered at startup. Results are cached so
//! repeated lookups across files are free.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::SystemTime;

use crate::checker::PackageOrigin;
use crate::namespace::parse_namespace_exports;
use rds2rust::{RObject, read_rds_from_path};

/// Information about an installed R package.
#[derive(Debug, Clone)]
pub struct PackageInfo {
    /// Exported function/object names from NAMESPACE.
    pub exports: HashSet<String>,
    /// Package version from DESCRIPTION (e.g., `(1, 2, 0)`).
    pub version: Option<(u32, u32, u32)>,
}

/// Lazily-populated cache of installed R package metadata.
///
/// Shared across all files being linted (wrapped in `Arc` for thread safety).
/// Packages are looked up on demand from the R library paths.
#[derive(Debug)]
pub struct PackageCache {
    /// Library paths discovered at startup (in priority order).
    library_paths: Vec<PathBuf>,
    /// Lazily populated: package name → info (`None` means looked up but not found).
    cache: RwLock<HashMap<String, Option<PackageInfo>>>,
    /// Modification times of package DESCRIPTION files at the time of lookup,
    /// used for staleness detection.
    mtimes: RwLock<HashMap<String, Option<SystemTime>>>,
}

impl PackageCache {
    pub fn new(library_paths: Vec<PathBuf>) -> Self {
        Self {
            library_paths,
            cache: RwLock::new(HashMap::new()),
            mtimes: RwLock::new(HashMap::new()),
        }
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
            };
            cache.insert(pkg_name.to_string(), Some(info));
        }
        Self {
            library_paths: Vec::new(),
            cache: RwLock::new(cache),
            mtimes: RwLock::new(HashMap::new()),
        }
    }

    /// Look up a package, reading from disk on first access.
    pub fn get(&self, name: &str) -> Option<PackageInfo> {
        // Fast path: already cached
        {
            let cache = self.cache.read().unwrap();
            if let Some(cached) = cache.get(name) {
                return cached.clone();
            }
        }

        // Slow path: look up on disk and cache
        let info = self.lookup_package(name);
        let mut cache = self.cache.write().unwrap();
        cache.insert(name.to_string(), info.clone());
        info
    }

    /// Check if any library paths are configured.
    pub fn is_available(&self) -> bool {
        !self.library_paths.is_empty()
    }

    /// Check whether any of the given packages have changed on disk since
    /// they were cached (by comparing DESCRIPTION mtime). Stale entries are
    /// evicted so the next `get()` call re-reads from disk.
    ///
    /// Returns the names of packages that were refreshed.
    pub fn refresh_if_stale(&self, packages: &[&str]) -> Vec<String> {
        let mut refreshed = Vec::new();
        let mtimes = self.mtimes.read().unwrap();

        for &pkg in packages {
            let Some(recorded) = mtimes.get(pkg) else {
                // Never looked up — nothing to refresh.
                continue;
            };
            let current = self.description_mtime(pkg);
            if current != *recorded {
                refreshed.push(pkg.to_string());
            }
        }
        drop(mtimes);

        if !refreshed.is_empty() {
            let mut cache = self.cache.write().unwrap();
            let mut mtimes = self.mtimes.write().unwrap();
            for pkg in &refreshed {
                cache.remove(pkg);
                mtimes.remove(pkg);
            }
        }

        refreshed
    }

    /// Get the mtime of a package's DESCRIPTION file (the most likely file to
    /// change on install/upgrade).
    fn description_mtime(&self, name: &str) -> Option<SystemTime> {
        for lib_path in &self.library_paths {
            let desc = lib_path.join(name).join("DESCRIPTION");
            if let Ok(meta) = std::fs::metadata(&desc) {
                return meta.modified().ok();
            }
        }
        None
    }

    /// Search library paths for a package and read its metadata.
    fn lookup_package(&self, name: &str) -> Option<PackageInfo> {
        for lib_path in &self.library_paths {
            let pkg_dir = lib_path.join(name);
            if !pkg_dir.is_dir() {
                continue;
            }

            let namespace_path = pkg_dir.join("NAMESPACE");
            let namespace_content = match std::fs::read_to_string(&namespace_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            // If NAMESPACE uses exportPattern, read the .rdx file to get all
            // object names so the pattern can be resolved.
            let all_names = if namespace_content.contains("exportPattern") {
                read_rdx_object_names(&pkg_dir, name)
            } else {
                Vec::new()
            };
            let all_name_refs: Vec<&str> = all_names.iter().map(|s| s.as_str()).collect();
            let exports = parse_namespace_exports(&namespace_content, &all_name_refs);

            let version = read_package_version(&pkg_dir);

            // Record the DESCRIPTION mtime for staleness detection.
            let desc_mtime = pkg_dir
                .join("DESCRIPTION")
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok());
            self.mtimes
                .write()
                .unwrap()
                .insert(name.to_string(), desc_mtime);

            return Some(PackageInfo { exports, version });
        }
        None
    }
}

/// Read all object names from a package's `.rdx` lazy-load database index.
///
/// The `.rdx` file is an RDS file containing a named list with a `$variables`
/// element. The names of `$variables` are all object names defined in the
/// package (both exported and internal).
fn read_rdx_object_names(pkg_dir: &Path, pkg_name: &str) -> Vec<String> {
    let rdx_path = pkg_dir.join("R").join(format!("{pkg_name}.rdx"));
    let parsed = match read_rds_from_path(&rdx_path) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    // The rdx is a named list: extract names from the `$variables` element.
    // Structure: List with "names" attribute = ["variables", "references", "compressed"]
    // We need the names attribute of the `variables` sub-list.
    extract_variables_names(&parsed.object).unwrap_or_default()
}

/// Extract the names from the `$variables` element of an rdx RObject.
///
/// The rdx parses as `WithAttributes { List([variables, references, compressed]),
/// names: ["variables", "references", "compressed"] }`.
/// The `variables` sub-element is itself `WithAttributes { List([...]),
/// names: [...all object names...] }`.
fn extract_variables_names(obj: &RObject) -> Option<Vec<String>> {
    let (items, attrs) = unwrap_list_with_attrs(obj)?;

    // Find the "variables" element by name
    let names = extract_string_vector(attrs.get("names")?)?;
    let var_idx = names.iter().position(|n| n == "variables")?;
    let variables_obj = items.get(var_idx)?;

    // Extract the names attribute from the variables sub-list
    let (_, var_attrs) = unwrap_list_with_attrs(variables_obj)?;
    extract_string_vector(var_attrs.get("names")?)
}

/// Unwrap an RObject that is a List with attributes (possibly wrapped in WithAttributes).
fn unwrap_list_with_attrs(obj: &RObject) -> Option<(&[RObject], &rds2rust::Attributes)> {
    if let RObject::WithAttributes { object, attributes } = obj
        && let RObject::List(items) = object.as_ref()
    {
        return Some((items, attributes));
    }
    None
}

/// Extract strings from an RObject that should be a character vector.
fn extract_string_vector(obj: &RObject) -> Option<Vec<String>> {
    if let RObject::Character(data) = obj {
        Some(data.as_vec().iter().map(|s| s.to_string()).collect())
    } else {
        None
    }
}

/// Read the `Version` field from a package's DESCRIPTION file.
fn read_package_version(pkg_dir: &Path) -> Option<(u32, u32, u32)> {
    let desc_path = pkg_dir.join("DESCRIPTION");
    let content = std::fs::read_to_string(desc_path).ok()?;

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("Version:") {
            let version_str = rest.trim();
            return parse_package_version(version_str);
        }
    }
    None
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_fake_package(lib_dir: &Path, name: &str, exports: &[&str], version: &str) {
        let pkg_dir = lib_dir.join(name);
        std::fs::create_dir_all(&pkg_dir).unwrap();

        // Write NAMESPACE
        let namespace_content: String = exports
            .iter()
            .map(|e| format!("export({e})"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(pkg_dir.join("NAMESPACE"), namespace_content).unwrap();

        // Write DESCRIPTION
        let desc = format!("Package: {name}\nVersion: {version}\n");
        std::fs::write(pkg_dir.join("DESCRIPTION"), desc).unwrap();
    }

    #[test]
    fn test_package_cache_lookup() {
        let dir = TempDir::new().unwrap();
        let lib_dir = dir.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();

        create_fake_package(&lib_dir, "dplyr", &["filter", "mutate", "select"], "1.1.4");

        let cache = PackageCache::new(vec![lib_dir]);

        let info = cache.get("dplyr").unwrap();
        assert!(info.exports.contains("filter"));
        assert!(info.exports.contains("mutate"));
        assert_eq!(info.version, Some((1, 1, 4)));

        // Second lookup should hit the cache
        let info2 = cache.get("dplyr").unwrap();
        assert!(info2.exports.contains("filter"));

        // Non-existent package
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_file_package_context_resolve() {
        let dir = TempDir::new().unwrap();
        let lib_dir = dir.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();

        create_fake_package(&lib_dir, "dplyr", &["filter", "mutate"], "1.1.4");
        create_fake_package(&lib_dir, "stats", &["filter", "lag"], "4.4.0");

        let cache = PackageCache::new(vec![lib_dir]);

        // library(stats) then library(dplyr) — both export `filter`
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
    fn test_export_pattern_with_rdx() {
        // Test against the real `astsa` package which uses:
        //   exportPattern("^[^\\.]")
        // This exports all objects not starting with a dot.
        let lib_dir = PathBuf::from("/home/etienne/R/x86_64-pc-linux-gnu-library/4.5");
        if !lib_dir.join("astsa").is_dir() {
            // Skip if astsa is not installed
            return;
        }

        let cache = PackageCache::new(vec![lib_dir]);
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

    #[test]
    fn test_parse_package_version() {
        assert_eq!(parse_package_version("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_package_version("1.2"), Some((1, 2, 0)));
        assert_eq!(parse_package_version("0.10.1"), Some((0, 10, 1)));
        assert_eq!(parse_package_version("1"), None);
    }
}
