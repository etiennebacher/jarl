//! Conversions between `lsp_types::Uri` and filesystem paths.
//!
//! Since lsp-types 0.97, the URI type is `lsp_types::Uri` (a thin wrapper
//! around `fluent_uri::Uri`) which, unlike the `url::Url` used by older
//! versions, does not provide `to_file_path` / `from_file_path`. We round-trip
//! through `url::Url`, which implements the platform-aware `file://`
//! conversions (drive letters, percent-encoding, UNC paths, ...).

use lsp_types::Uri;
use std::path::{Path, PathBuf};

/// Convert a `file://` URI into a filesystem path, if it denotes a local file.
pub fn uri_to_file_path(uri: &Uri) -> Option<PathBuf> {
    url::Url::parse(uri.as_str()).ok()?.to_file_path().ok()
}

/// Build a `file://` URI from a filesystem path.
pub fn file_path_to_uri(path: &Path) -> Option<Uri> {
    let url = url::Url::from_file_path(path).ok()?;
    url.as_str().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_file_path_through_uri() {
        let dir = tempfile::tempdir().unwrap();
        // The space exercises percent-encoding on the way out and decoding back.
        let path = dir.path().join("test file.R");

        let uri = file_path_to_uri(&path).expect("path should convert to a file URI");
        assert!(uri.as_str().starts_with("file://"));

        let back = uri_to_file_path(&uri).expect("file URI should convert back to a path");
        assert_eq!(back, path);
    }

    #[test]
    fn non_file_uri_has_no_file_path() {
        let uri: Uri = "untitled:Untitled-1".parse().unwrap();
        assert!(uri_to_file_path(&uri).is_none());
    }
}
