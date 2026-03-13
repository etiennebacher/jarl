//! R availability check.
//!
//! Provides a quick check for whether R is installed, used by the CLI
//! to give a clear error when package-specific rules are enabled but R
//! is not available.

use std::path::PathBuf;
use std::process::Command;

/// Check whether R is available on this system.
///
/// First checks the `R_HOME` environment variable (set by CI setup actions
/// like `r-lib/actions/setup-r`). Falls back to running `R RHOME` which
/// works on all platforms where R is installed.
pub fn is_r_available() -> bool {
    // Fast path: check env var (always set in CI with setup-r)
    if let Ok(home) = std::env::var("R_HOME")
        && !home.is_empty()
        && PathBuf::from(&home).is_dir()
    {
        return true;
    }

    // Fallback: ask R itself (works when R is on PATH but R_HOME isn't set)
    Command::new("R")
        .arg("RHOME")
        .output()
        .is_ok_and(|o| o.status.success())
}
