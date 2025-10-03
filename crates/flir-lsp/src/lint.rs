//! Core linting integration for the Flir LSP server
//!
//! This module provides the minimal bridge between the LSP server and your Flir linting engine.
//! It focuses purely on running your linter and converting results to LSP diagnostics.
//! No code actions, fixes, or other advanced features - just highlighting issues.

use anyhow::{Result, anyhow};
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

use std::path::Path;

use crate::DIAGNOSTIC_SOURCE;
use crate::document::PositionEncoding;
use crate::session::DocumentSnapshot;

use air_workspace::resolve::PathResolver;
use flir_core::discovery::{DiscoveredSettings, discover_r_file_paths, discover_settings};
use flir_core::{
    config::ArgsConfig, config::build_config, diagnostic::Diagnostic as FlirDiagnostic,
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
fn run_flir_linting(_content: &str, file_path: Option<&Path>) -> Result<Vec<FlirDiagnostic>> {
    let file_path = match file_path {
        Some(path) => path,
        None => {
            tracing::warn!("No file path provided for linting");
            return Ok(Vec::new());
        }
    };

    let path_str = match file_path.to_str() {
        Some(s) => s.to_string(),
        None => {
            tracing::warn!("File path contains invalid UTF-8: {:?}", file_path);
            return Ok(Vec::new());
        }
    };

    tracing::debug!("Linting file: {}", path_str);
    let path: Vec<String> = vec![path_str];

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
        .flat_map(|(_, result)| match result {
            Ok(diags) => {
                tracing::debug!("Found {} diagnostics for file", diags.len());
                diags
            }
            Err(e) => {
                tracing::error!("Error checking file: {}", e);
                Vec::new()
            }
        })
        .collect();

    tracing::debug!("Total diagnostics: {}", all_diagnostics.len());

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
    // Use the TextRange from the diagnostic for accurate positioning
    let text_range = flir_diag.range;
    let start_offset = text_range.start().into();
    let end_offset = text_range.end().into();

    let start_pos = byte_offset_to_lsp_position(start_offset, content, encoding)?;
    let end_pos = byte_offset_to_lsp_position(end_offset, content, encoding)?;

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

/// Convert byte offset to LSP Position
fn byte_offset_to_lsp_position(
    byte_offset: usize,
    content: &str,
    encoding: PositionEncoding,
) -> Result<Position> {
    if byte_offset > content.len() {
        return Err(anyhow!(
            "Byte offset {} is out of bounds (max {})",
            byte_offset,
            content.len()
        ));
    }

    // Find the line number and column by counting from the start
    let mut current_offset = 0;
    let mut line = 0;

    for line_content in content.lines() {
        let line_start = current_offset;
        let line_end = current_offset + line_content.len();

        if byte_offset <= line_end {
            // Found the line containing this offset
            let column_byte_offset = byte_offset - line_start;

            // Convert byte offset within the line to the appropriate character offset
            let lsp_character = match encoding {
                PositionEncoding::UTF8 => column_byte_offset as u32,
                PositionEncoding::UTF16 => {
                    // Convert from byte offset to UTF-16 code unit offset
                    let prefix = &line_content[..column_byte_offset.min(line_content.len())];
                    prefix.chars().map(|c| c.len_utf16()).sum::<usize>() as u32
                }
                PositionEncoding::UTF32 => {
                    // Convert from byte offset to Unicode scalar value offset
                    let prefix = &line_content[..column_byte_offset.min(line_content.len())];
                    prefix.chars().count() as u32
                }
            };

            return Ok(Position::new(line as u32, lsp_character));
        }

        // Move to the next line (add 1 for the newline character)
        current_offset = line_end + 1;
        line += 1;
    }

    // If we get here, the offset was at the very end of the file
    Ok(Position::new(line as u32, 0))
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
        let uri = Url::parse("file:///test.R").unwrap();
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
    fn test_position_conversion() {
        let content = "hello\nworld\ntest";

        // Test basic position conversion using byte offsets
        let pos = byte_offset_to_lsp_position(7, content, PositionEncoding::UTF8).unwrap(); // "w" in "world"
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 1);

        // Test start of file
        let pos = byte_offset_to_lsp_position(0, content, PositionEncoding::UTF8).unwrap();
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);

        // Test end of file
        let pos =
            byte_offset_to_lsp_position(content.len(), content, PositionEncoding::UTF8).unwrap();
        assert_eq!(pos.line, 2);
        assert_eq!(pos.character, 4); // After "test"

        // Test out of bounds
        assert!(byte_offset_to_lsp_position(1000, content, PositionEncoding::UTF8).is_err());
    }

    #[test]
    fn test_unicode_handling() {
        let content = "hello 🌍 world";

        // Test UTF-16 encoding with emoji
        // The emoji 🌍 starts at byte offset 6
        let pos = byte_offset_to_lsp_position(6, content, PositionEncoding::UTF16).unwrap();
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 6); // 6 UTF-16 code units: "hello "

        // Test UTF-8 encoding
        let pos_utf8 = byte_offset_to_lsp_position(6, content, PositionEncoding::UTF8).unwrap();
        assert_eq!(pos_utf8.line, 0);
        assert_eq!(pos_utf8.character, 6); // 6 bytes: "hello "

        // Test UTF-32 encoding
        let pos_utf32 = byte_offset_to_lsp_position(6, content, PositionEncoding::UTF32).unwrap();
        assert_eq!(pos_utf32.line, 0);
        assert_eq!(pos_utf32.character, 6); // 6 Unicode scalar values: "hello "
    }
}
