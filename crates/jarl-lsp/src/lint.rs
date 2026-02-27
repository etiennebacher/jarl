//! Core linting integration for the Jarl LSP server
//!
//! This module provides the bridge between the LSP server and your Jarl linting engine.
//! It handles diagnostics, code actions, and fixes for automatic issue resolution.

use anyhow::{Result, anyhow};
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use serde::{Deserialize, Serialize};

use std::path::{Path, PathBuf};

use crate::DIAGNOSTIC_SOURCE;
use crate::document::PositionEncoding;
use crate::session::DocumentSnapshot;
use crate::utils::should_exclude_file_based_on_settings;

use air_workspace::resolve::PathResolver;
use jarl_core::check::get_checks;
use jarl_core::config::{ArgsConfig, build_config};
use jarl_core::diagnostic::Diagnostic as JarlDiagnostic;
use jarl_core::discovery::{DiscoveredSettings, discover_settings};
use jarl_core::fs::{has_r_extension, relativize_path};
use jarl_core::package::{compute_package_analysis, is_in_r_package};
use jarl_core::settings::Settings;

/// Fix information that can be attached to a diagnostic for code actions
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiagnosticFix {
    /// The replacement content for the fix
    pub content: String,
    /// The start byte offset of the fix range
    pub start: usize,
    /// The end byte offset of the fix range
    pub end: usize,
    /// Whether this fix is safe to apply automatically
    pub is_safe: bool,
    /// The name of the rule that produced this diagnostic
    pub rule_name: String,
    /// The start byte offset of the diagnostic range (for suppression insertion)
    pub diagnostic_start: usize,
    /// The end byte offset of the diagnostic range (for suppression insertion)
    pub diagnostic_end: usize,
}

/// Result of linting a document, including diagnostics and information about
/// hidden unused_function diagnostics.
pub struct LintOutput {
    pub diagnostics: Vec<Diagnostic>,
    /// Number of unused_function diagnostics hidden because the package-wide
    /// count exceeded `threshold-ignore`. Zero if none were hidden.
    pub unused_fn_hidden_count: usize,
}

/// Main entry point for linting a document
///
/// Takes a document snapshot, runs your Jarl linter, and returns LSP diagnostics
/// for highlighting issues in the editor. The diagnostics include fix information
/// that can be used for code actions if needed.
pub fn lint_document(snapshot: &DocumentSnapshot) -> Result<LintOutput> {
    let content = snapshot.content();
    let file_path = snapshot.file_path();
    let encoding = snapshot.position_encoding();

    // Run the actual linting
    let LintInternalOutput {
        diagnostics: jarl_diagnostics,
        unused_fn_hidden_count,
    } = run_jarl_linting(content, file_path.as_deref())?;

    // Convert to LSP diagnostics with fix information
    let mut lsp_diagnostics = Vec::new();
    for jarl_diagnostic in jarl_diagnostics {
        let lsp_diagnostic = convert_to_lsp_diagnostic(&jarl_diagnostic, content, encoding)?;
        lsp_diagnostics.push(lsp_diagnostic);
    }

    Ok(LintOutput {
        diagnostics: lsp_diagnostics,
        unused_fn_hidden_count,
    })
}

struct LintInternalOutput {
    diagnostics: Vec<JarlDiagnostic>,
    unused_fn_hidden_count: usize,
}

/// Run the Jarl linting engine on the given content
fn run_jarl_linting(content: &str, file_path: Option<&Path>) -> Result<LintInternalOutput> {
    let empty = LintInternalOutput { diagnostics: Vec::new(), unused_fn_hidden_count: 0 };

    let file_path = match file_path {
        Some(path) => path,
        None => {
            tracing::warn!("No file path provided for linting");
            return Ok(empty);
        }
    };

    if file_path.to_str().is_none() {
        tracing::warn!("File path contains invalid UTF-8: {:?}", file_path);
        return Ok(empty);
    }

    // Discover settings from the actual file path.
    let actual_file_path = vec![file_path.to_string_lossy().to_string()];
    let mut resolver = PathResolver::new(Settings::default());
    for DiscoveredSettings { directory, settings, .. } in discover_settings(&actual_file_path)? {
        resolver.add(&directory, settings);
        tracing::debug!("Discovered settings from directory: {:?}", directory);
    }

    // Check if the file should be excluded based on settings in jarl.toml
    // (`exclude` or `default-exclude`).
    if should_exclude_file_based_on_settings(file_path, &resolver) {
        tracing::debug!("Skipping linting for excluded file: {:?}", file_path);
        return Ok(empty);
    }

    let check_config = ArgsConfig {
        files: vec![file_path.to_path_buf()],
        fix: false,
        unsafe_fixes: false,
        fix_only: false,
        select: "".to_string(),
        extend_select: "".to_string(),
        ignore: "".to_string(),
        min_r_version: None,
        allow_dirty: false,
        allow_no_vcs: false,
        assignment: None,
    };

    let toml_settings = resolver.items().first().map(|item| item.value());
    let config = build_config(&check_config, toml_settings, vec![file_path.to_path_buf()])?;

    // Compute package-level analysis using the real file's sibling R files.
    let pkg = if let Some(package_files) = collect_sibling_r_files(file_path) {
        compute_package_analysis(&package_files, &config)
    } else {
        Default::default()
    };

    // Call get_checks directly with the in-memory content and the real
    // (relativized) file path, avoiding the old tempfile round-trip.
    let rel_path = PathBuf::from(relativize_path(file_path));
    let mut diagnostics = get_checks(content, &rel_path, &config, &pkg)?;

    // Hide unused_function diagnostics when the package-wide count exceeds
    // the threshold, matching the CLI behaviour. The LSP never passes
    // `--select unused_function` so we only check the toml settings.
    let unused_fn_hidden_count = {
        let explicitly_selected = resolver.items().iter().any(|item| {
            let linter = &item.value().linter;
            linter
                .select
                .iter()
                .chain(linter.extend_select.iter())
                .flatten()
                .any(|s| s == "unused_function")
        });

        if explicitly_selected {
            0
        } else {
            let total_unused: usize = pkg.unused_functions.values().map(|v| v.len()).sum();
            let threshold = resolver
                .items()
                .iter()
                .map(|item| {
                    item.value()
                        .linter
                        .rule_options
                        .unused_function
                        .threshold_ignore
                })
                .min()
                .unwrap_or(50);

            if total_unused > threshold {
                diagnostics.retain(|d| d.message.name != "unused_function");
                total_unused
            } else {
                0
            }
        }
    };

    tracing::debug!("Found {} diagnostics for file", diagnostics.len());
    Ok(LintInternalOutput { diagnostics, unused_fn_hidden_count })
}

/// If `file_path` lives inside an R package's `R/` directory, return all
/// `.R` files in that directory. Returns `None` otherwise.
fn collect_sibling_r_files(file_path: &Path) -> Option<Vec<PathBuf>> {
    if !is_in_r_package(file_path).unwrap_or(false) {
        return None;
    }
    let r_dir = file_path.parent()?;
    let entries = std::fs::read_dir(r_dir).ok()?;
    let files = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| has_r_extension(p))
        .collect();
    Some(files)
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
        diagnostic_start: start_offset,
        diagnostic_end: end_offset,
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
    use tempfile::TempDir;

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
        let output = lint_document(&snapshot).unwrap();
        assert!(output.diagnostics.is_empty());
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
        );

        // Should return no diagnostics because file is excluded
        let output = lint_document(&snapshot).unwrap();
        assert!(
            output.diagnostics.is_empty(),
            "Expected no diagnostics but got: {:?}",
            output.diagnostics
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
        );

        // Should return diagnostics because file is not excluded
        let diagnostics = lint_document(&snapshot).unwrap().diagnostics;
        assert!(
            !diagnostics.is_empty(),
            "Expected a diagnostic but didn't get any"
        );

        Ok(())
    }

    // --- Rmd/Qmd indentation tests ---

    /// Lint Rmd/Qmd content and return the LSP diagnostics.
    ///
    /// Writes the content to a real temporary file so that `Url::from_file_path`
    /// produces a valid URI on all platforms (including Windows, where a fake
    /// `file:///test.Rmd` path would cause `to_file_path()` to fail and return
    /// no diagnostics).
    fn lint_rmd_content(content: &str, ext: &str) -> Vec<lsp_types::Diagnostic> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(format!("test.{ext}"));
        std::fs::write(&file_path, content).unwrap();

        let uri = lsp_types::Url::from_file_path(&file_path).unwrap();
        let key = DocumentKey::from(uri);
        let document = TextDocument::new(content.to_string(), 1);
        let snapshot = DocumentSnapshot::new(
            document,
            key,
            PositionEncoding::UTF8,
            ClientCapabilities::default(),
        );
        lint_document(&snapshot).unwrap().diagnostics
    }

    /// Filter diagnostics by rule name stored in the `data` field.
    fn diagnostics_for_rule<'a>(
        diagnostics: &'a [lsp_types::Diagnostic],
        rule: &str,
    ) -> Vec<&'a lsp_types::Diagnostic> {
        diagnostics
            .iter()
            .filter(|d| {
                d.data
                    .as_ref()
                    .and_then(|v| v.get("rule_name"))
                    .and_then(|r| r.as_str())
                    == Some(rule)
            })
            .collect()
    }

    #[test]
    fn test_rmd_non_indented_chunk_diagnostic_position() {
        // Non-indented chunk: diagnostic should land at column 0.
        //
        // Line 0: "---"
        // Line 1: "---"
        // Line 2: ""
        // Line 3: "```{r}"
        // Line 4: "any(is.na(x))"  <- violation here (column 0)
        // Line 5: "```"
        let content = "---\n---\n\n```{r}\nany(is.na(x))\n```\n";
        let diagnostics = lint_rmd_content(content, "Rmd");

        let hits = diagnostics_for_rule(&diagnostics, "any_is_na");
        assert_eq!(hits.len(), 1, "expected exactly one any_is_na diagnostic");

        let range = hits[0].range;
        assert_eq!(range.start.line, 4, "diagnostic should be on line 4");
        assert_eq!(
            range.start.character, 0,
            "diagnostic should start at column 0"
        );
    }

    #[test]
    fn test_rmd_indented_chunk_diagnostic_column() {
        // 2-space indented chunk (typical list-item scenario).
        //
        // Line 0: "* hello"
        // Line 1: ""
        // Line 2: "  ```{r}"
        // Line 3: "  any(is.na(x))"  <- violation here (column 2, after the indent)
        // Line 4: "  ```"
        let content = "* hello\n\n  ```{r}\n  any(is.na(x))\n  ```\n";
        let diagnostics = lint_rmd_content(content, "Rmd");

        let hits = diagnostics_for_rule(&diagnostics, "any_is_na");
        assert_eq!(hits.len(), 1, "expected exactly one any_is_na diagnostic");

        let range = hits[0].range;
        assert_eq!(range.start.line, 3, "diagnostic should be on line 3");
        assert_eq!(
            range.start.character, 2,
            "diagnostic column must account for the 2-space indent"
        );
    }

    #[test]
    fn test_rmd_indented_chunk_second_line_violation() {
        // Indented chunk where the violation is on the second line of code.
        //
        // Line 0: "* item"
        // Line 1: ""
        // Line 2: "  ```{r}"
        // Line 3: "  x <- 1"        (clean)
        // Line 4: "  any(is.na(x))"  <- violation here (line 4, column 2)
        // Line 5: "  ```"
        let content = "* item\n\n  ```{r}\n  x <- 1\n  any(is.na(x))\n  ```\n";
        let diagnostics = lint_rmd_content(content, "Rmd");

        let hits = diagnostics_for_rule(&diagnostics, "any_is_na");
        assert_eq!(hits.len(), 1, "expected exactly one any_is_na diagnostic");

        let range = hits[0].range;
        assert_eq!(range.start.line, 4, "diagnostic should be on line 4");
        assert_eq!(
            range.start.character, 2,
            "diagnostic column must account for the 2-space indent"
        );
    }

    #[test]
    fn test_rmd_tab_indented_chunk_diagnostic_column() {
        // Tab-indented chunk: diagnostic column should be 1 (one tab = one byte in UTF-8).
        //
        // Line 0: "\t```{r}"
        // Line 1: "\tany(is.na(x))"  <- violation here (column 1, after the tab)
        // Line 2: "\t```"
        let content = "\t```{r}\n\tany(is.na(x))\n\t```\n";
        let diagnostics = lint_rmd_content(content, "Rmd");

        let hits = diagnostics_for_rule(&diagnostics, "any_is_na");
        assert_eq!(hits.len(), 1, "expected exactly one any_is_na diagnostic");

        let range = hits[0].range;
        assert_eq!(range.start.line, 1, "diagnostic should be on line 1");
        assert_eq!(
            range.start.character, 1,
            "diagnostic column must account for the tab indent (1 byte)"
        );
    }

    #[test]
    fn test_qmd_indented_chunk_diagnostic_column() {
        // Same indented-chunk test but with a .qmd extension.
        //
        // Line 0: "* hello"
        // Line 1: ""
        // Line 2: "  ```{r}"
        // Line 3: "  any(is.na(x))"  <- violation here (column 2)
        // Line 4: "  ```"
        let content = "* hello\n\n  ```{r}\n  any(is.na(x))\n  ```\n";
        let diagnostics = lint_rmd_content(content, "qmd");

        let hits = diagnostics_for_rule(&diagnostics, "any_is_na");
        assert_eq!(hits.len(), 1, "expected exactly one any_is_na diagnostic");

        let range = hits[0].range;
        assert_eq!(range.start.line, 3, "diagnostic should be on line 3");
        assert_eq!(
            range.start.character, 2,
            "diagnostic column must account for the 2-space indent"
        );
    }

    // --- Package-level duplicate function definition tests ---

    /// Create a minimal R package in a temp dir and return (temp_dir, R/ path).
    /// The TempDir is returned to keep it alive for the duration of the test.
    fn create_test_package() -> (TempDir, std::path::PathBuf) {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("DESCRIPTION"),
            "Package: testpkg\nVersion: 0.1.0\n",
        )
        .unwrap();
        let r_dir = dir.path().join("R");
        std::fs::create_dir(&r_dir).unwrap();
        (dir, r_dir)
    }

    /// Create a snapshot backed by a real file on disk.
    fn create_snapshot_for_file(file_path: &std::path::Path, content: &str) -> DocumentSnapshot {
        let uri = Url::from_file_path(file_path).unwrap();
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
    fn test_duplicate_function_same_file() {
        let (_dir, r_dir) = create_test_package();

        let content = "foo <- function() 1\nfoo <- function() 2\n";
        let file = r_dir.join("foo.R");
        std::fs::write(&file, content).unwrap();

        let snapshot = create_snapshot_for_file(&file, content);
        let diagnostics = lint_document(&snapshot).unwrap().diagnostics;

        let hits = diagnostics_for_rule(&diagnostics, "duplicated_function_definition");
        assert_eq!(
            hits.len(),
            1,
            "expected one duplicate diagnostic, got: {hits:?}"
        );
        assert!(
            hits[0].message.contains("`foo` is defined more than once"),
            "message should mention `foo`, got: {}",
            hits[0].message
        );
    }

    #[test]
    fn test_duplicate_function_cross_file() {
        let (_dir, r_dir) = create_test_package();

        // aaa.R defines `foo` first (alphabetically), bbb.R is the duplicate
        std::fs::write(r_dir.join("aaa.R"), "foo <- function() 1\n").unwrap();
        let content = "foo <- function() 2\n";
        let file = r_dir.join("bbb.R");
        std::fs::write(&file, content).unwrap();

        let snapshot = create_snapshot_for_file(&file, content);
        let diagnostics = lint_document(&snapshot).unwrap().diagnostics;

        let hits = diagnostics_for_rule(&diagnostics, "duplicated_function_definition");
        assert_eq!(
            hits.len(),
            1,
            "expected one duplicate diagnostic on bbb.R, got: {hits:?}"
        );
    }

    #[test]
    fn test_no_duplicate_outside_package() {
        // No DESCRIPTION so not a package, rule should not fire
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        std::fs::create_dir(&r_dir).unwrap();

        let content = "foo <- function() 1\nfoo <- function() 2\n";
        let file = r_dir.join("foo.R");
        std::fs::write(&file, content).unwrap();

        let snapshot = create_snapshot_for_file(&file, content);
        let diagnostics = lint_document(&snapshot).unwrap().diagnostics;

        let hits = diagnostics_for_rule(&diagnostics, "duplicated_function_definition");
        assert!(
            hits.is_empty(),
            "should not flag duplicates outside a package, got: {hits:?}"
        );
    }

    #[test]
    fn test_no_duplicate_unique_names() {
        let (_dir, r_dir) = create_test_package();

        std::fs::write(r_dir.join("aaa.R"), "foo <- function() 1\n").unwrap();
        let content = "bar <- function() 2\n";
        let file = r_dir.join("bbb.R");
        std::fs::write(&file, content).unwrap();

        let snapshot = create_snapshot_for_file(&file, content);
        let diagnostics = lint_document(&snapshot).unwrap().diagnostics;

        let hits = diagnostics_for_rule(&diagnostics, "duplicated_function_definition");
        assert!(
            hits.is_empty(),
            "unique names should not be flagged, got: {hits:?}"
        );
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
        );

        // Should return no diagnostics because file matches exclude pattern
        let diagnostics = lint_document(&snapshot).unwrap().diagnostics;
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics for excluded generated file, but got: {:?}",
            diagnostics
        );

        Ok(())
    }
}
