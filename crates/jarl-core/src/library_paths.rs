//! R library path discovery.
//!
//! Discovers where R packages are installed so that the linter can look up
//! package metadata (NAMESPACE, DESCRIPTION) for package-specific rules.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Discover R library paths for package lookups.
///
/// Strategy:
/// 1. If the project uses renv, the library path is deterministic (no R needed).
/// 2. Otherwise, run `Rscript -e 'cat(.libPaths(), sep = "\n")'` once.
///
/// Returns an empty vector if R is not available. This is safe — package-
/// specific rules simply won't fire.
pub fn discover_library_paths(project_root: Option<&Path>) -> Vec<PathBuf> {
    if let Some(root) = project_root
        && let Some(mut paths) = discover_renv_library(root)
    {
        // renv only provides user package paths. Append the system library
        // (where base/recommended packages like stats live) so that
        // package resolution works for all packages.
        if let Some(system_paths) = discover_system_library() {
            for p in system_paths {
                if !paths.contains(&p) {
                    paths.push(p);
                }
            }
        }
        return paths;
    }

    discover_via_rscript().unwrap_or_default()
}

/// Detect renv and return its library path.
///
/// renv stores packages in `renv/library/<platform>/<R-version>/`.
fn discover_renv_library(project_root: &Path) -> Option<Vec<PathBuf>> {
    let has_renv = project_root.join("renv.lock").exists() || project_root.join("renv").is_dir();
    if !has_renv {
        return None;
    }

    let renv_lib = project_root.join("renv").join("library");
    if !renv_lib.is_dir() {
        return None;
    }

    // renv/library/<platform>/<R-version>/
    let mut paths = Vec::new();
    if let Ok(platforms) = std::fs::read_dir(&renv_lib) {
        for platform_entry in platforms.flatten() {
            let platform_path = platform_entry.path();
            if platform_path.is_dir()
                && let Ok(versions) = std::fs::read_dir(&platform_path)
            {
                for version_entry in versions.flatten() {
                    let version_path = version_entry.path();
                    if version_path.is_dir() {
                        paths.push(version_path);
                    }
                }
            }
        }
    }

    if paths.is_empty() { None } else { Some(paths) }
}

/// Discover the system R library path (where base/recommended packages live).
///
/// This is the path returned by `R.home("library")`, e.g.
/// `/opt/R/4.5.0/lib/R/library`. We try to find it without running R by
/// checking the `R_HOME` environment variable, falling back to `Rscript`.
fn discover_system_library() -> Option<Vec<PathBuf>> {
    // Try R_HOME first (avoids spawning a process)
    if let Ok(r_home) = std::env::var("R_HOME") {
        let lib_path = PathBuf::from(r_home).join("library");
        if lib_path.is_dir() {
            return Some(vec![lib_path]);
        }
    }

    // Fall back to Rscript
    let output = Command::new("Rscript")
        .args(["-e", "cat(R.home(\"library\"))"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let path = PathBuf::from(stdout.trim());
    if path.is_dir() {
        Some(vec![path])
    } else {
        None
    }
}

/// Run `Rscript` to discover library paths.
fn discover_via_rscript() -> Option<Vec<PathBuf>> {
    let output = Command::new("Rscript")
        .args(["-e", "cat(.libPaths(), sep = \"\\n\")"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let paths: Vec<PathBuf> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .collect();

    if paths.is_empty() { None } else { Some(paths) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_no_renv_returns_none() {
        let dir = TempDir::new().unwrap();
        assert!(discover_renv_library(dir.path()).is_none());
    }

    #[test]
    fn test_renv_without_library_dir_returns_none() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("renv.lock"), "{}").unwrap();
        assert!(discover_renv_library(dir.path()).is_none());
    }

    #[test]
    fn test_renv_with_library_returns_paths() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("renv.lock"), "{}").unwrap();
        let lib_path = dir
            .path()
            .join("renv")
            .join("library")
            .join("x86_64-pc-linux-gnu")
            .join("4.4");
        std::fs::create_dir_all(&lib_path).unwrap();

        let paths = discover_renv_library(dir.path()).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], lib_path);
    }
}
