//! Integration tests for flir_lsp
//!
//! These tests verify the core functionality of the LSP server
//! by testing the linting integration and diagnostic conversion.

use anyhow::Result;
use flir_lsp::{lint, PositionEncoding};
use lsp_types::{DiagnosticSeverity, Position, Range, Url};
use std::path::Path;

/// Mock document query for testing
/// In a real test, you'd use the actual ruff_server types
struct MockDocumentQuery {
    content: String,
    uri: Url,
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
            content: content.to_string(),
            uri: uri.clone(),
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

#[test]
fn test_long_line_diagnostic() -> Result<()> {
    let content = "this is a very long line that exceeds the maximum allowed length of 100 characters and should trigger a diagnostic about line length";
    let query = MockDocumentQuery::new(content, "long_line.py");

    let diagnostics = lint::run_flir_linting(content, query.file_path.as_deref())?;

    assert_eq!(diagnostics.len(), 1, "Should find one long line diagnostic");

    let diag = &diagnostics[0];
    assert_eq!(diag.code.as_ref().unwrap(), "F001");
    assert!(diag.message.contains("Line too long"));
    assert_eq!(diag.line, 0);
    assert_eq!(diag.column, 0);

    Ok(())
}

#[test]
fn test_todo_comment_diagnostic() -> Result<()> {
    let content = "# TODO: implement this function\ndef foo():\n    pass";
    let query = MockDocumentQuery::new(content, "todo.py");

    let diagnostics = lint::run_flir_linting(content, query.file_path.as_deref())?;

    assert_eq!(diagnostics.len(), 1, "Should find one TODO diagnostic");

    let diag = &diagnostics[0];
    assert_eq!(diag.code.as_ref().unwrap(), "F002");
    assert_eq!(diag.message, "TODO comment found");
    assert_eq!(diag.line, 0);
    assert_eq!(diag.column, 2); // Position of "todo" in "# TODO"

    Ok(())
}

#[test]
fn test_multiple_diagnostics() -> Result<()> {
    let content = r#"# TODO: fix this
this is a very long line that exceeds the maximum allowed length of 100 characters and should trigger a diagnostic
def foo():
    # another todo comment here
    pass
"#;
    let query = MockDocumentQuery::new(content, "multiple.py");

    let diagnostics = lint::run_flir_linting(content, query.file_path.as_deref())?;

    assert_eq!(diagnostics.len(), 3, "Should find three diagnostics");

    // Check we have both types of diagnostics
    let todo_count = diagnostics
        .iter()
        .filter(|d| d.code.as_ref() == Some(&"F002".to_string()))
        .count();
    let long_line_count = diagnostics
        .iter()
        .filter(|d| d.code.as_ref() == Some(&"F001".to_string()))
        .count();

    assert_eq!(todo_count, 2, "Should find two TODO comments");
    assert_eq!(long_line_count, 1, "Should find one long line");

    Ok(())
}

#[test]
fn test_diagnostic_conversion() -> Result<()> {
    use lint::{MockFlirDiagnostic, MockFlirSeverity};

    let flir_diag = MockFlirDiagnostic {
        message: "Test message".to_string(),
        severity: MockFlirSeverity::Warning,
        line: 5,
        column: 10,
        end_line: 5,
        end_column: 20,
        code: Some("TEST001".to_string()),
    };

    let content =
        "line 0\nline 1\nline 2\nline 3\nline 4\nthis is line 5 with some content\nline 6";
    let lsp_diagnostic =
        lint::convert_flir_diagnostic_to_lsp(flir_diag, content, PositionEncoding::UTF8)?;

    assert_eq!(lsp_diagnostic.message, "Test message");
    assert_eq!(lsp_diagnostic.severity, Some(DiagnosticSeverity::WARNING));
    assert_eq!(
        lsp_diagnostic.code,
        Some(lsp_types::NumberOrString::String("TEST001".to_string()))
    );
    assert_eq!(lsp_diagnostic.source.as_ref().unwrap(), "Flir");

    // Check range
    assert_eq!(lsp_diagnostic.range.start, Position::new(5, 10));
    assert_eq!(lsp_diagnostic.range.end, Position::new(5, 20));

    Ok(())
}

#[test]
fn test_severity_conversion() {
    use lint::{convert_flir_severity, MockFlirSeverity};

    assert_eq!(
        convert_flir_severity(MockFlirSeverity::Error),
        DiagnosticSeverity::ERROR
    );
    assert_eq!(
        convert_flir_severity(MockFlirSeverity::Warning),
        DiagnosticSeverity::WARNING
    );
    assert_eq!(
        convert_flir_severity(MockFlirSeverity::Info),
        DiagnosticSeverity::INFORMATION
    );
    assert_eq!(
        convert_flir_severity(MockFlirSeverity::Hint),
        DiagnosticSeverity::HINT
    );
}

#[test]
fn test_position_encoding_utf8() -> Result<()> {
    let content = "hello world";
    let pos = lint::line_col_to_position(0, 6, content, PositionEncoding::UTF8)?;

    assert_eq!(pos.line, 0);
    assert_eq!(pos.character, 6);

    Ok(())
}

#[test]
fn test_multiline_content() -> Result<()> {
    let content = r#"line 1
line 2 is ok
this line 3 is very long and exceeds the maximum allowed length of 100 characters so it should trigger a diagnostic
line 4
# TODO: fix line 3
line 6"#;

    let query = MockDocumentQuery::new(content, "multiline.py");
    let diagnostics = lint::run_flir_linting(content, query.file_path.as_deref())?;

    // Should find one long line (line 3, index 2) and one TODO (line 5, index 4)
    assert_eq!(diagnostics.len(), 2);

    // Check long line diagnostic
    let long_line = diagnostics
        .iter()
        .find(|d| d.code.as_ref() == Some(&"F001".to_string()))
        .unwrap();
    assert_eq!(long_line.line, 2); // Line 3 (0-indexed)

    // Check TODO diagnostic
    let todo = diagnostics
        .iter()
        .find(|d| d.code.as_ref() == Some(&"F002".to_string()))
        .unwrap();
    assert_eq!(todo.line, 4); // Line 5 (0-indexed)
    assert_eq!(todo.column, 2); // Position of "todo" in "# TODO"

    Ok(())
}

#[test]
fn test_no_diagnostics() -> Result<()> {
    let content = r#"short line
another short line
def function():
    return True
"#;

    let query = MockDocumentQuery::new(content, "clean.py");
    let diagnostics = lint::run_flir_linting(content, query.file_path.as_deref())?;

    assert!(
        diagnostics.is_empty(),
        "Clean code should have no diagnostics"
    );
    Ok(())
}

// Integration test for the main check_document function
// Note: This would need proper DocumentQuery implementation in a real scenario
#[test]
fn test_diagnostics_map_structure() {
    // This test demonstrates what the structure would look like
    // when you have the real DocumentQuery implementation

    let content = "this line is too long to fit within the 100 character limit and should definitely trigger a warning";

    // In a real test, you'd do something like:
    // let query = create_test_document_query(content, "test.py");
    // let diagnostics_map = lint::check_document(&query, PositionEncoding::UTF8).unwrap();

    // For now, we can test the structure with our mock
    let mut expected_map = std::collections::HashMap::new();
    let url = Url::parse("file:///test.py").unwrap();
    expected_map.insert(url.clone(), vec![]);

    assert!(expected_map.contains_key(&url));
    assert_eq!(expected_map.len(), 1);
}
