//! Core linting integration for the Jarl LSP server
//!
//! This module provides the bridge between the LSP server and your Jarl linting engine.
//! It handles diagnostics, code actions, and fixes for automatic issue resolution.

use anyhow::{Result, anyhow};
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use std::path::Path;

use crate::DIAGNOSTIC_SOURCE;
use crate::document::PositionEncoding;
use crate::session::DocumentSnapshot;
use crate::utils::should_exclude_file_based_on_settings;

use air_workspace::resolve::PathResolver;
use jarl_core::discovery::{DiscoveredSettings, discover_r_file_paths, discover_settings};
use jarl_core::{
    config::ArgsConfig, config::build_config, diagnostic::Diagnostic as JarlDiagnostic,
    settings::Settings,
};

/// Fix information that can be attached to a diagnostic for code actions
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiagnosticFix {
    pub content: String,
    pub start: usize,
    pub end: usize,
    pub is_safe: bool,
    pub rule_name: String,
}

/// Main entry point for linting a document
///
/// Takes a document snapshot, runs your Jarl linter, and returns LSP diagnostics
/// for highlighting issues in the editor. The diagnostics include fix information
/// that can be used for code actions if needed.
pub fn lint_document(snapshot: &DocumentSnapshot) -> Result<Vec<Diagnostic>> {
    let content = snapshot.content();
    let file_path = snapshot.file_path();
    let encoding = snapshot.position_encoding();
    let assignment_operator = snapshot.assignment_operator();

    // Run the actual linting
    let jarl_diagnostics = run_jarl_linting(content, file_path.as_deref(), assignment_operator)?;

    // Convert to LSP diagnostics with fix information
    let mut lsp_diagnostics = Vec::new();
    for jarl_diagnostic in jarl_diagnostics {
        let lsp_diagnostic = convert_to_lsp_diagnostic(&jarl_diagnostic, content, encoding)?;
        lsp_diagnostics.push(lsp_diagnostic);
    }

    Ok(lsp_diagnostics)
}

/// Run the Jarl linting engine on the given content
fn run_jarl_linting(
    content: &str,
    file_path: Option<&Path>,
    lsp_assignment_operator: Option<&String>,
) -> Result<Vec<JarlDiagnostic>> {
    let file_path = match file_path {
        Some(path) => path,
        None => {
            tracing::warn!("No file path provided for linting");
            return Ok(Vec::new());
        }
    };

    if file_path.to_str().is_none() {
        tracing::warn!("File path contains invalid UTF-8: {:?}", file_path);
        return Ok(Vec::new());
    }

    // Discover settings from the actual file path.
    let actual_file_path = vec![file_path.to_string_lossy().to_string()];
    let mut resolver = PathResolver::new(Settings::default());
    for DiscoveredSettings { directory, settings } in discover_settings(&actual_file_path)? {
        resolver.add(&directory, settings);
        tracing::debug!("Discovered settings from directory: {:?}", directory);
    }

    // Check if the file should be excluded based on settings in jarl.toml
    // (`exclude` or `default-exclude`).
    if should_exclude_file_based_on_settings(file_path, &resolver) {
        tracing::debug!("Skipping linting for excluded file: {:?}", file_path);
        return Ok(Vec::new());
    }

    // TODO: we shoudln't have to write the content to a tempfile to then read
    // it and get diagnostic. The check function should be able to take the R
    // code as a string.
    // Write in-memory content to a temporary file for linting
    let temp_dir = TempDir::new()?;
    let temp_dir = temp_dir.path();
    let temp_file = temp_dir.join(format!("jarl_lsp_{}.R", std::process::id()));

    std::fs::write(&temp_file, content)
        .map_err(|e| anyhow!("Failed to write temporary file: {}", e))?;
    let temp_path_str = temp_file.to_string_lossy().to_string();
    let temp_path: Vec<String> = vec![temp_path_str];

    // Use temp path for discovering R file paths (just the temp file itself)
    let paths = discover_r_file_paths(&temp_path, &resolver, true)
        .into_iter()
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    // Check if TOML has assignment setting and if so use it, otherwise use
    // the assignment from the workspace settings.
    let toml_has_assignment = resolver
        .items()
        .iter()
        .any(|item| item.value().linter.assignment.is_some());

    let assignment_operator = if toml_has_assignment {
        None
    } else {
        lsp_assignment_operator.cloned()
    };

    let check_config = ArgsConfig {
        files: temp_path.iter().map(|s| s.into()).collect(),
        fix: false,
        unsafe_fixes: false,
        fix_only: false,
        select_rules: "".to_string(),
        extend_select: "".to_string(),
        ignore_rules: "".to_string(),
        min_r_version: None,
        allow_dirty: false,
        allow_no_vcs: false,
        assignment_op: assignment_operator,
    };

    let config = build_config(&check_config, &resolver, paths)?;

    let diagnostics = jarl_core::check::check(config);
    let mut all_diagnostics: Vec<JarlDiagnostic> = diagnostics
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

    // Clean up temporary file
    if let Err(e) = std::fs::remove_file(&temp_file) {
        tracing::warn!("Failed to remove temporary file {:?}: {}", temp_file, e);
    }

    // Update diagnostics to point to the original file instead of temp file
    for diagnostic in &mut all_diagnostics {
        diagnostic.filename = file_path.to_path_buf();
    }

    Ok(all_diagnostics)
}

/// Convert a Jarl diagnostic to LSP diagnostic format with fix information
fn convert_to_lsp_diagnostic(
    jarl_diag: &JarlDiagnostic,
    content: &str,
    encoding: PositionEncoding,
) -> Result<Diagnostic> {
    // Use the TextRange from the diagnostic for accurate positioning
    let text_range = jarl_diag.range;
    let start_offset = text_range.start().into();
    let end_offset = text_range.end().into();

    let start_pos = byte_offset_to_lsp_position(start_offset, content, encoding)?;
    let end_pos = byte_offset_to_lsp_position(end_offset, content, encoding)?;

    let range = Range::new(start_pos, end_pos);

    // TODO-etienne: don't have that
    // let severity = convert_severity(jarl_diag.severity);
    let severity = DiagnosticSeverity::WARNING;

    // Extract fix information if available
    // Always include fix_data even if there's no actual fix, so we can access the rule_name
    let diagnostic_fix = DiagnosticFix {
        content: jarl_diag.fix.content.clone(),
        start: jarl_diag.fix.start,
        end: jarl_diag.fix.end,
        is_safe: jarl_diag.has_safe_fix(),
        rule_name: jarl_diag.message.name.clone(),
    };
    let fix_data = Some(serde_json::to_value(diagnostic_fix).unwrap_or_default());

    // Build the LSP diagnostic with fix information
    // Combine body and suggestion for the message
    let message = if let Some(suggestion) = &jarl_diag.message.suggestion {
        format!("{} {}", jarl_diag.message.body, suggestion)
    } else {
        jarl_diag.message.body.clone()
    };

    let diagnostic = Diagnostic {
        range,
        severity: Some(severity),
        code: None,
        code_description: None,
        source: Some(DIAGNOSTIC_SOURCE.to_string()),
        message,
        related_information: None,
        tags: None,
        data: fix_data, // Include fix information for code actions when available
    };

    Ok(diagnostic)
}

/// Convert byte offset to LSP Position (made public for code actions)
pub fn byte_offset_to_lsp_position(
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

    // Find the line number and column by iterating through the content
    let mut line = 0;
    let mut line_start_offset = 0;

    // Iterate through the content to find line breaks
    for (i, ch) in content.char_indices() {
        if i >= byte_offset {
            // We've passed the target offset, so we're on the current line
            let column_byte_offset = byte_offset - line_start_offset;
            let line_content = &content[line_start_offset..];

            // Find the end of the current line
            let line_end = line_content.find('\n').unwrap_or(line_content.len());
            let line_str = &line_content[..line_end];

            // Convert byte offset within the line to the appropriate character offset
            let lsp_character = match encoding {
                PositionEncoding::UTF8 => column_byte_offset as u32,
                PositionEncoding::UTF16 => {
                    // Convert from byte offset to UTF-16 code unit offset
                    let prefix = &line_str[..column_byte_offset.min(line_str.len())];
                    prefix.chars().map(|c| c.len_utf16()).sum::<usize>() as u32
                }
                PositionEncoding::UTF32 => {
                    // Convert from byte offset to Unicode scalar value offset
                    let prefix = &line_str[..column_byte_offset.min(line_str.len())];
                    prefix.chars().count() as u32
                }
            };

            return Ok(Position::new(line as u32, lsp_character));
        }

        if ch == '\n' {
            line += 1;
            // The next line starts right after this newline character
            // char_indices gives us the byte offset of the current char,
            // so the next char starts at i + ch.len_utf8()
            line_start_offset = i + ch.len_utf8();
        }
    }

    // If we get here, the offset is at the very end of the file
    let column_byte_offset = byte_offset - line_start_offset;
    let line_content = &content[line_start_offset..];

    let lsp_character = match encoding {
        PositionEncoding::UTF8 => column_byte_offset as u32,
        PositionEncoding::UTF16 => {
            line_content.chars().map(|c| c.len_utf16()).sum::<usize>() as u32
        }
        PositionEncoding::UTF32 => line_content.chars().count() as u32,
    };

    Ok(Position::new(line as u32, lsp_character))
}

// /// Convert Jarl severity to LSP diagnostic severity
// fn convert_severity(severity: JarlSeverity) -> DiagnosticSeverity {
//     match severity {
//         JarlSeverity::Error => DiagnosticSeverity::ERROR,
//         JarlSeverity::Warning => DiagnosticSeverity::WARNING,
//         JarlSeverity::Info => DiagnosticSeverity::INFORMATION,
//         JarlSeverity::Hint => DiagnosticSeverity::HINT,
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
            None,
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

    #[test]
    fn test_multiline_with_empty_lines() {
        let content = "any(is.na(x))\n\nany(is.na(y))";

        // Position 0 should be line 0, col 0
        let pos = byte_offset_to_lsp_position(0, content, PositionEncoding::UTF8).unwrap();
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);

        // Position 13 is the first newline
        let pos = byte_offset_to_lsp_position(13, content, PositionEncoding::UTF8).unwrap();
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 13);

        // Position 14 is the second newline (empty line)
        let pos = byte_offset_to_lsp_position(14, content, PositionEncoding::UTF8).unwrap();
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);

        // Position 15 is the start of "any(is.na(y))" - should be line 2, col 0
        let pos = byte_offset_to_lsp_position(15, content, PositionEncoding::UTF8).unwrap();
        assert_eq!(pos.line, 2);
        assert_eq!(pos.character, 0);

        // Position 16 is 'n' in the second "any" - should be line 2, col 1
        let pos = byte_offset_to_lsp_position(16, content, PositionEncoding::UTF8).unwrap();
        assert_eq!(pos.line, 2);
        assert_eq!(pos.character, 1);
    }

    #[test]
    fn test_exclusion_with_default_exclude() -> Result<(), Box<dyn std::error::Error>> {
        let directory = TempDir::new()?;
        let directory = directory.path();

        std::fs::write(
            directory.join("jarl.toml"),
            r#"
    [lint]
    "#,
        )
        .unwrap();

        // Create a file that has violations but should be ignored
        let file_path = directory.join("import-standalone-foo.R");
        let content = "any(is.na(x))";
        std::fs::write(&file_path, content).unwrap();

        // Create snapshot for the renv file
        let uri = lsp_types::Url::from_file_path(&file_path).unwrap();
        let key = crate::document::DocumentKey::from(uri);
        let document = crate::document::TextDocument::new(content.to_string(), 1);
        let snapshot = DocumentSnapshot::new(
            document,
            key,
            PositionEncoding::UTF8,
            lsp_types::ClientCapabilities::default(),
            None,
        );

        // Should return no diagnostics because file is excluded
        let diagnostics = lint_document(&snapshot).unwrap();
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics but got: {:?}",
            diagnostics
        );

        Ok(())
    }

    #[test]
    fn test_exclusion_disabled_default_exclude() -> Result<(), Box<dyn std::error::Error>> {
        let directory = TempDir::new()?;
        let directory = directory.path();

        std::fs::write(
            directory.join("jarl.toml"),
            r#"
    [lint]
    default-exclude = false
    "#,
        )
        .unwrap();

        // Create a file that has violations and would be ignored if we had
        // `default-exclude = true`.
        let file_path = directory.join("import-standalone-hello-there.R");
        println!("file_path: {:?}", file_path);
        let content = "any(is.na(x))\n";
        std::fs::write(&file_path, content).unwrap();

        // Create snapshot for the renv file
        let uri = lsp_types::Url::from_file_path(&file_path).unwrap();
        let key = crate::document::DocumentKey::from(uri);
        let document = crate::document::TextDocument::new(content.to_string(), 1);
        let snapshot = DocumentSnapshot::new(
            document,
            key,
            PositionEncoding::UTF8,
            lsp_types::ClientCapabilities::default(),
            None,
        );

        // Should return diagnostics because file is not excluded
        let diagnostics = lint_document(&snapshot).unwrap();
        assert!(
            !diagnostics.is_empty(),
            "Expected a diagnostic but didn't get any"
        );

        Ok(())
    }

    #[test]
    fn test_exclusion_with_custom_exclude_pattern() -> Result<(), Box<dyn std::error::Error>> {
        let directory = TempDir::new()?;
        let directory = directory.path();

        std::fs::write(
            directory.join("jarl.toml"),
            r#"
    [lint]
    exclude = ["generated-*"]
    "#,
        )
        .unwrap();

        // Create a file matching the custom exclude pattern
        let file_path = directory.join("generated-code.R");
        let content = "any(is.na())";
        std::fs::write(&file_path, content).unwrap();

        // Create snapshot for the generated file
        let uri = lsp_types::Url::from_file_path(&file_path).unwrap();
        let key = crate::document::DocumentKey::from(uri);
        let document = crate::document::TextDocument::new(content.to_string(), 1);
        let snapshot = DocumentSnapshot::new(
            document,
            key,
            PositionEncoding::UTF8,
            lsp_types::ClientCapabilities::default(),
            None,
        );

        // Should return no diagnostics because file matches exclude pattern
        let diagnostics = lint_document(&snapshot).unwrap();
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics for excluded generated file, but got: {:?}",
            diagnostics
        );

        Ok(())
    }
}
