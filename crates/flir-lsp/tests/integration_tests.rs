//! Integration tests for flir_lsp
//!
//! These tests verify the core functionality of the LSP server
//! by testing the linting integration and diagnostic conversion.

use anyhow::Result;
use flir_lsp::lint;
use lsp_types::Url;
use std::path::Path;

/// Mock document query for testing
/// In a real test, you'd use the actual ruff_server types
struct MockDocumentQuery {
    _content: String,
    _uri: Url,
    file_path: Option<std::path::PathBuf>,
}

impl MockDocumentQuery {
    fn new(content: &str, file_name: &str) -> Self {
        let uri = Url::from_file_path(
            std::env::current_dir()
                .unwrap()
                .join("test_files")
                .join(file_name),
        )
        .unwrap();

        Self {
            _content: content.to_string(),
            _uri: uri.clone(),
            file_path: Some(Path::new(file_name).to_path_buf()),
        }
    }
}

// Note: This is a simplified mock. In reality, you'd implement the actual
// DocumentQuery trait from ruff_server or create a proper test harness.

#[test]
fn test_empty_document() -> Result<()> {
    let content = "";
    let query = MockDocumentQuery::new(content, "empty.py");

    // In a real test, you'd call: lint::check_document(&query, PositionEncoding::UTF8)
    // For now, we test the mock linting function directly
    let diagnostics = lint::run_flir_linting(content, query.file_path.as_deref())?;

    assert!(
        diagnostics.is_empty(),
        "Empty document should have no diagnostics"
    );
    Ok(())
}
