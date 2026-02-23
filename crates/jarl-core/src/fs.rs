//
// Taken from Ark
// https://github.com/posit-dev/air/blob/affa92cd514525c4bab6c8c2ca251ea19414b89f/crates/fs/src/lib.rs
//
// MIT License - Posit PBC

use path_absolutize::Absolutize;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

pub fn has_r_extension(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(is_r_extension)
}

pub fn is_r_extension(extension: &str) -> bool {
    matches!(extension, "r" | "R")
}

pub fn has_rmd_extension(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(is_rmd_extension)
}

pub fn is_rmd_extension(ext: &str) -> bool {
    matches!(ext, "rmd" | "Rmd" | "qmd" | "Qmd")
}

/// Convert any path to an absolute path (based on the current working
/// directory).
pub fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = path.as_ref();
    if let Ok(path) = path.absolutize() {
        path.to_path_buf()
    } else {
        path.to_path_buf()
    }
}

/// Convert an absolute path to be relative to the current working directory.
pub fn relativize_path<P: AsRef<Path>>(path: P) -> String {
    let path = path.as_ref();

    #[cfg(target_arch = "wasm32")]
    let cwd = Path::new(".");
    #[cfg(not(target_arch = "wasm32"))]
    let cwd = path_absolutize::path_dedot::CWD.as_path();

    if let Ok(path) = path.strip_prefix(cwd) {
        return format!("{}", path.display());
    }
    format!("{}", path.display())
}
