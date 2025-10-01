//! Core linting integration for the Flir LSP server
//!
//! This module provides the minimal bridge between the LSP server and your Flir linting engine.
//! It focuses purely on running your linter and converting results to LSP diagnostics.
//! No code actions, fixes, or other advanced features - just highlighting issues.

use anyhow::{anyhow, Result};
use flir_core::location::Location;
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};
use std::collections::HashMap;
use std::path::Path;

use crate::document::PositionEncoding;
use crate::session::DocumentSnapshot;
use crate::DIAGNOSTIC_SOURCE;

use air_workspace::resolve::PathResolver;
use flir_core::discovery::{discover_r_file_paths, discover_settings, DiscoveredSettings};
use flir_core::{
    config::build_config, config::ArgsConfig, diagnostic::Diagnostic as FlirDiagnostic,
    settings::Settings,
};

// TODO: Replace these imports with your actual flir_core types:
// use flir_core::{Linter, Config, Diagnostic as FlirCoreDiagnostic, Level};

/// Main entry point for linting a document
///
/// Takes a document snapshot, runs your Flir linter, and returns LSP diagnostics
/// for highlighting issues in the editor. This is the core function of the LSP server.
pub fn lint_document(snapshot: &DocumentSnapshot) -> Result<Vec<Diagnostic>> {
    let content = snapshot.content();
    let file_path = snapshot.file_path();
    let encoding = snapshot.position_encoding();

    // Run the actual linting
    let flir_diagnostics = run_flir_linting(content, file_path.as_deref())?;

    // Convert to LSP diagnostics
    let mut lsp_diagnostics = Vec::new();
    for flir_diagnostic in flir_diagnostics {
        let lsp_diagnostic = convert_to_lsp_diagnostic(flir_diagnostic, content, encoding)?;
        lsp_diagnostics.push(lsp_diagnostic);
    }

    Ok(lsp_diagnostics)
}

/// Run the Flir linting engine on the given content
///
/// TODO: Replace this mock implementation with actual calls to your flir_core crate:
///
/// ```rust
/// fn run_flir_linting(content: &str, file_path: Option<&Path>) -> Result<Vec<FlirDiagnostic>> {
///     let config = flir_core::Config::load_from_path(file_path)?;
///     let linter = flir_core::Linter::new(config);
///     let results = linter.lint_text(content, file_path)?;
///
///     Ok(results.into_iter().map(|d| FlirDiagnostic {
///         message: d.message,
///         severity: match d.level {
///             flir_core::Level::Error => FlirSeverity::Error,
///             flir_core::Level::Warning => FlirSeverity::Warning,
///             // ... etc
///         },
///         line: d.span.start.line,
///         column: d.span.start.column,
///         end_line: d.span.end.line,
///         end_column: d.span.end.column,
///         code: Some(d.rule_code),
///         rule_name: Some(d.rule_name),
///     }).collect())
/// }
/// ```
fn run_flir_linting(content: &str, file_path: Option<&Path>) -> Result<Vec<FlirDiagnostic>> {
    let path: Vec<String> = vec![file_path.unwrap().to_str().unwrap().to_string()];

    let mut resolver = PathResolver::new(Settings::default());
    for DiscoveredSettings { directory, settings } in discover_settings(&path)? {
        resolver.add(&directory, settings);
    }

    let paths = discover_r_file_paths(&path, &resolver, true)
        .into_iter()
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    let check_config = ArgsConfig {
        files: path.iter().map(|s| s.into()).collect(),
        fix: false,
        unsafe_fixes: false,
        fix_only: false,
        select_rules: "".to_string(),
        ignore_rules: "".to_string(),
        min_r_version: None,
    };

    let config = build_config(&check_config, &resolver, paths)?;

    let diagnostics = flir_core::check::check(config);
    let all_diagnostics: Vec<FlirDiagnostic> = diagnostics
        .into_iter()
        .flat_map(|(_, result)| result.unwrap_or_default())
        .collect();

    tracing::debug!(
        "Flir linting completed for {:?}: {} diagnostics found",
        file_path,
        all_diagnostics.len()
    );

    Ok(all_diagnostics)
}

/// Convert a Flir diagnostic to LSP diagnostic format
fn convert_to_lsp_diagnostic(
    flir_diag: FlirDiagnostic,
    content: &str,
    encoding: PositionEncoding,
) -> Result<Diagnostic> {
    let start_pos = line_col_to_lsp_position(
        flir_diag
            .location
            .unwrap_or(Location::new(0, 0))
            .row()
            .try_into()
            .unwrap(),
        flir_diag
            .location
            .unwrap_or(Location::new(0, 0))
            .column()
            .try_into()
            .unwrap(),
        content,
        encoding,
    )?;
    // TODO-etienne: need the infrastructure for that
    // let end_pos = line_col_to_lsp_position(flir_diag.end_line, flir_diag.end_column, content, encoding)?;
    let end_pos = Position::new(start_pos.line + 5, start_pos.character + 5);

    let range = Range::new(start_pos, end_pos);

    // TODO-etienne: don't have that
    // let severity = convert_severity(flir_diag.severity);
    let severity = DiagnosticSeverity::WARNING;

    // Build the LSP diagnostic (no code actions or fixes - just highlighting)
    let diagnostic = Diagnostic {
        range,
        severity: Some(severity),
        code: Some(lsp_types::NumberOrString::String(flir_diag.message.name)),
        code_description: None,
        source: Some(DIAGNOSTIC_SOURCE.to_string()),
        message: flir_diag.message.body,
        related_information: None,
        tags: None,
        data: None, // No fix data needed for diagnostics-only mode
    };

    Ok(diagnostic)
}

/// Convert line/column coordinates to LSP Position
fn line_col_to_lsp_position(
    line: u32,
    column: u32,
    content: &str,
    encoding: PositionEncoding,
) -> Result<Position> {
    let line_index = line as usize;
    let lines: Vec<&str> = content.lines().collect();

    if line_index >= lines.len() {
        return Err(anyhow!(
            "Line {} is out of bounds (max {})",
            line,
            lines.len()
        ));
    }

    let line_content = lines[line_index];
    let column_byte = column as usize;

    if column_byte > line_content.len() {
        return Err(anyhow!(
            "Column {} is out of bounds for line {} (max {})",
            column,
            line,
            line_content.len()
        ));
    }

    let lsp_character = match encoding {
        PositionEncoding::UTF8 => column,
        PositionEncoding::UTF16 => {
            // Convert from byte offset to UTF-16 code unit offset
            let prefix = &line_content[..column_byte.min(line_content.len())];
            prefix.chars().map(|c| c.len_utf16()).sum::<usize>() as u32
        }
        PositionEncoding::UTF32 => {
            // Convert from byte offset to Unicode scalar value offset
            let prefix = &line_content[..column_byte.min(line_content.len())];
            prefix.chars().count() as u32
        }
    };

    Ok(Position::new(line, lsp_character))
}

// /// Convert Flir severity to LSP diagnostic severity
// fn convert_severity(severity: FlirSeverity) -> DiagnosticSeverity {
//     match severity {
//         FlirSeverity::Error => DiagnosticSeverity::ERROR,
//         FlirSeverity::Warning => DiagnosticSeverity::WARNING,
//         FlirSeverity::Info => DiagnosticSeverity::INFORMATION,
//         FlirSeverity::Hint => DiagnosticSeverity::HINT,
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{DocumentKey, TextDocument};
    use crate::session::DocumentSnapshot;
    use lsp_types::{ClientCapabilities, Url};

    fn create_test_snapshot(content: &str) -> DocumentSnapshot {
        let uri = Url::parse("file:///test.py").unwrap();
        let key = DocumentKey::from(uri);
        let document = TextDocument::new(content.to_string(), 1);

        DocumentSnapshot::new(
            document,
            key,
            PositionEncoding::UTF8,
            ClientCapabilities::default(),
        )
    }

    #[test]
    fn test_empty_document() {
        let snapshot = create_test_snapshot("");
        let diagnostics = lint_document(&snapshot).unwrap();
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_long_line_detection() {
        let long_line = "a".repeat(150); // 150 characters, over the 100 limit
        let snapshot = create_test_snapshot(&long_line);
        let diagnostics = lint_document(&snapshot).unwrap();

        assert_eq!(diagnostics.len(), 1);
        let diag = &diagnostics[0];
        assert!(diag.message.contains("Line too long"));
        assert_eq!(diag.severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(diag.range.start.line, 0);
        assert_eq!(diag.range.start.character, 0);
    }

    #[test]
    fn test_todo_comment_detection() {
        let content = "# TODO: implement this\nprint('hello')";
        let snapshot = create_test_snapshot(content);
        let diagnostics = lint_document(&snapshot).unwrap();

        assert_eq!(diagnostics.len(), 1);
        let diag = &diagnostics[0];
        assert!(diag.message.contains("TODO comment"));
        assert_eq!(diag.severity, Some(DiagnosticSeverity::INFORMATION));
        assert_eq!(diag.range.start.line, 0);
        assert_eq!(diag.range.start.character, 2); // Position of "todo" in "# TODO"
    }

    #[test]
    fn test_trailing_whitespace_detection() {
        let content = "print('hello')   \nprint('world')";
        let snapshot = create_test_snapshot(content);
        let diagnostics = lint_document(&snapshot).unwrap();

        assert_eq!(diagnostics.len(), 1);
        let diag = &diagnostics[0];
        assert!(diag.message.contains("Trailing whitespace"));
        assert_eq!(diag.severity, Some(DiagnosticSeverity::INFORMATION));
        assert_eq!(diag.range.start.line, 0);
        assert_eq!(diag.range.start.character, 14); // After "print('hello')"
    }

    #[test]
    fn test_multiple_issues() {
        let content = r#"# TODO: fix this line that is way too long and exceeds the maximum allowed length of 100 characters which should trigger multiple diagnostics
print('hello world')
"#;
        let snapshot = create_test_snapshot(content);
        let diagnostics = lint_document(&snapshot).unwrap();

        // Should have: long line + TODO + trailing whitespace = 3 diagnostics
        assert_eq!(diagnostics.len(), 3);

        let codes: Vec<_> = diagnostics
            .iter()
            .filter_map(|d| d.code.as_ref())
            .filter_map(|c| match c {
                lsp_types::NumberOrString::String(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();

        assert!(codes.contains(&"FLIR001")); // long line
        assert!(codes.contains(&"FLIR002")); // TODO
        assert!(codes.contains(&"FLIR003")); // trailing whitespace
    }

    #[test]
    fn test_position_conversion() {
        let content = "hello\nworld\ntest";

        // Test basic position conversion
        let pos = line_col_to_lsp_position(1, 2, content, PositionEncoding::UTF8).unwrap();
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 2);

        // Test out of bounds
        assert!(line_col_to_lsp_position(10, 0, content, PositionEncoding::UTF8).is_err());
        assert!(line_col_to_lsp_position(0, 100, content, PositionEncoding::UTF8).is_err());
    }

    #[test]
    fn test_unicode_handling() {
        let content = "hello 🌍 world";

        // Test UTF-16 encoding with emoji
        let pos = line_col_to_lsp_position(0, 6, content, PositionEncoding::UTF16).unwrap();
        assert_eq!(pos.line, 0);
        // The emoji 🌍 takes 2 UTF-16 code units, but we're asking for byte position 6
        // which should be converted appropriately
        assert_eq!(pos.character, 6);

        // Test UTF-8 encoding
        let pos_utf8 = line_col_to_lsp_position(0, 6, content, PositionEncoding::UTF8).unwrap();
        assert_eq!(pos_utf8.line, 0);
        assert_eq!(pos_utf8.character, 6);
    }
}
