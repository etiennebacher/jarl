//! Lazy cache of installed R package metadata.
//!
//! Looks up package NAMESPACE (exports) and DESCRIPTION (version) on demand
//! from the R library paths discovered at startup. Results are cached so
//! repeated lookups across files are free.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::namespace::parse_namespace_exports;

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
}

impl PackageCache {
    pub fn new(library_paths: Vec<PathBuf>) -> Self {
        Self { library_paths, cache: RwLock::new(HashMap::new()) }
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
            // Skip exportPattern resolution for external packages (no object list available)
            let exports = parse_namespace_exports(&namespace_content, &[]);

            let version = read_package_version(&pkg_dir);

            return Some(PackageInfo { exports, version });
        }
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

    /// Resolve which package a bare function name likely comes from.
    ///
    /// Walks loaded packages in **reverse** order (last loaded wins,
    /// matching R's masking behavior). Returns `None` if not found in
    /// any loaded package.
    pub fn resolve_package(&self, fn_name: &str) -> Option<&str> {
        for pkg_name in self.loaded_packages.iter().rev() {
            if let Some(info) = self.cache.get(pkg_name)
                && info.exports.contains(fn_name)
            {
                return Some(pkg_name);
            }
        }
        None
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

        // library(stats) then library(dplyr) — dplyr masks stats::filter
        let ctx = FilePackageContext::new(vec!["stats".to_string(), "dplyr".to_string()], &cache);
        assert_eq!(ctx.resolve_package("filter"), Some("dplyr"));
        assert_eq!(ctx.resolve_package("lag"), Some("stats"));
        assert_eq!(ctx.resolve_package("nonexistent"), None);
    }

    #[test]
    fn test_parse_package_version() {
        assert_eq!(parse_package_version("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_package_version("1.2"), Some((1, 2, 0)));
        assert_eq!(parse_package_version("0.10.1"), Some((0, 10, 1)));
        assert_eq!(parse_package_version("1"), None);
    }
}
