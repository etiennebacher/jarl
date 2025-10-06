//! Document management for the Flir LSP server
//!
//! This module handles document lifecycle, content tracking, and position encoding.

use anyhow::{Result, anyhow};
use lsp_types::{Position, Range, TextDocumentContentChangeEvent, Url};
use std::path::PathBuf;

/// Position encoding supported by the LSP server
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PositionEncoding {
    /// UTF-8 encoding (each character is a UTF-8 code unit)
    UTF8,
    /// UTF-16 encoding (each character is a UTF-16 code unit) - LSP default
    #[default]
    UTF16,
    /// UTF-32 encoding (each character is a UTF-32 code unit)
    UTF32,
}

impl From<PositionEncoding> for lsp_types::PositionEncodingKind {
    fn from(encoding: PositionEncoding) -> Self {
        match encoding {
            PositionEncoding::UTF8 => lsp_types::PositionEncodingKind::UTF8,
            PositionEncoding::UTF16 => lsp_types::PositionEncodingKind::UTF16,
            PositionEncoding::UTF32 => lsp_types::PositionEncodingKind::UTF32,
        }
    }
}

impl TryFrom<&lsp_types::PositionEncodingKind> for PositionEncoding {
    type Error = anyhow::Error;

    fn try_from(kind: &lsp_types::PositionEncodingKind) -> Result<Self> {
        if kind == &lsp_types::PositionEncodingKind::UTF8 {
            Ok(PositionEncoding::UTF8)
        } else if kind == &lsp_types::PositionEncodingKind::UTF16 {
            Ok(PositionEncoding::UTF16)
        } else if kind == &lsp_types::PositionEncodingKind::UTF32 {
            Ok(PositionEncoding::UTF32)
        } else {
            Err(anyhow!("Unsupported position encoding: {:?}", kind))
        }
    }
}

/// Unique key for identifying documents
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocumentKey {
    /// The URI of the document
    uri: Url,
}

impl DocumentKey {
    pub fn new(uri: Url) -> Self {
        Self { uri }
    }

    pub fn uri(&self) -> &Url {
        &self.uri
    }

    pub fn into_url(self) -> Url {
        self.uri
    }

    /// Try to get the file path if this is a file:// URI
    pub fn file_path(&self) -> Option<PathBuf> {
        self.uri.to_file_path().ok()
    }
}

impl From<Url> for DocumentKey {
    fn from(uri: Url) -> Self {
        Self::new(uri)
    }
}

/// Version of a document, used for tracking changes
pub type DocumentVersion = i32;

/// A text document managed by the LSP server
#[derive(Debug, Clone)]
pub struct TextDocument {
    /// The content of the document
    content: String,
    /// The version of the document
    version: DocumentVersion,
    /// The language ID of the document (e.g., "python")
    language_id: Option<String>,
    /// Cached line starts for efficient position calculations
    line_starts: Vec<usize>,
}

impl TextDocument {
    /// Create a new text document
    pub fn new(content: String, version: DocumentVersion) -> Self {
        let line_starts = Self::compute_line_starts(&content);
        Self { content, version, language_id: None, line_starts }
    }

    /// Set the language ID for this document
    pub fn with_language_id(mut self, language_id: &str) -> Self {
        self.language_id = Some(language_id.to_string());
        self
    }

    /// Get the content of the document
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the version of the document
    pub fn version(&self) -> DocumentVersion {
        self.version
    }

    /// Get the language ID of the document
    pub fn language_id(&self) -> Option<&str> {
        self.language_id.as_deref()
    }

    /// Apply incremental changes to the document
    pub fn apply_changes(
        &mut self,
        changes: Vec<TextDocumentContentChangeEvent>,
        new_version: DocumentVersion,
        encoding: PositionEncoding,
    ) -> Result<()> {
        tracing::debug!(
            "Applying {} changes to document, new version: {}",
            changes.len(),
            new_version
        );

        // Convert positions to offsets first, then sort by offset in reverse order
        let mut changes_with_offsets = Vec::new();
        for change in changes {
            let range = change.range.ok_or_else(|| {
                anyhow!(
                    "Full document replacement not supported - only incremental changes allowed"
                )
            })?;

            let start_offset = self.position_to_offset(range.start, encoding)?;
            let end_offset = self.position_to_offset(range.end, encoding)?;
            changes_with_offsets.push((start_offset, end_offset, change));
        }

        // Sort by start offset in reverse order (end to beginning) to avoid offset invalidation
        changes_with_offsets.sort_by(|a, b| b.0.cmp(&a.0));

        // Apply incremental changes in reverse order
        for (i, (start_offset, end_offset, change)) in changes_with_offsets.iter().enumerate() {
            tracing::trace!(
                "Processing incremental change {}: {}..{} -> '{}'",
                i,
                start_offset,
                end_offset,
                change.text
            );

            if *start_offset > self.content.len() || *end_offset > self.content.len() {
                return Err(anyhow!(
                    "Change range is out of bounds: {}..{} in document of length {}",
                    start_offset,
                    end_offset,
                    self.content.len()
                ));
            }

            if start_offset > end_offset {
                return Err(anyhow!(
                    "Invalid change range: start {} > end {}",
                    start_offset,
                    end_offset
                ));
            }

            self.content
                .replace_range(*start_offset..*end_offset, &change.text);
        }

        self.version = new_version;
        self.line_starts = Self::compute_line_starts(&self.content);
        Ok(())
    }

    /// Convert a Position to a byte offset in the document
    pub fn position_to_offset(
        &self,
        position: Position,
        encoding: PositionEncoding,
    ) -> Result<usize> {
        let line = position.line as usize;
        let character = position.character as usize;

        if line >= self.line_starts.len() {
            return Err(anyhow!("Line {} is out of bounds", line));
        }

        let line_start = self.line_starts[line];
        let line_end = if line + 1 < self.line_starts.len() {
            self.line_starts[line + 1] - 1 // -1 to exclude the newline
        } else {
            self.content.len()
        };

        let line_content = &self.content[line_start..line_end];

        let offset = match encoding {
            PositionEncoding::UTF8 => {
                if character > line_content.len() {
                    return Err(anyhow!(
                        "Character {} is out of bounds on line {}",
                        character,
                        line
                    ));
                }
                character
            }
            PositionEncoding::UTF16 => {
                let mut utf16_pos = 0;
                let mut byte_pos = 0;
                for ch in line_content.chars() {
                    if utf16_pos >= character {
                        break;
                    }
                    utf16_pos += ch.len_utf16();
                    byte_pos += ch.len_utf8();
                }
                if utf16_pos > character {
                    return Err(anyhow!(
                        "Character {} is in the middle of a UTF-16 code unit",
                        character
                    ));
                }
                byte_pos
            }
            PositionEncoding::UTF32 => {
                let mut char_count = 0;
                let mut byte_pos = 0;
                for ch in line_content.chars() {
                    if char_count >= character {
                        break;
                    }
                    char_count += 1;
                    byte_pos += ch.len_utf8();
                }
                byte_pos
            }
        };

        Ok(line_start + offset)
    }

    /// Convert a byte offset to a Position
    pub fn offset_to_position(
        &self,
        offset: usize,
        encoding: PositionEncoding,
    ) -> Result<Position> {
        if offset > self.content.len() {
            return Err(anyhow!("Offset {} is out of bounds", offset));
        }

        // Binary search to find the line
        let line = match self.line_starts.binary_search(&offset) {
            Ok(line) => line,
            Err(line) => {
                if line == 0 {
                    0
                } else {
                    line - 1
                }
            }
        };

        let line_start = self.line_starts[line];
        let line_end = if line + 1 < self.line_starts.len() {
            self.line_starts[line + 1] - 1
        } else {
            self.content.len()
        };

        let line_content = &self.content[line_start..line_end];
        let byte_offset_in_line = offset - line_start;

        let character = match encoding {
            PositionEncoding::UTF8 => byte_offset_in_line,
            PositionEncoding::UTF16 => {
                let prefix = &line_content[..byte_offset_in_line.min(line_content.len())];
                prefix.chars().map(|c| c.len_utf16()).sum::<usize>()
            }
            PositionEncoding::UTF32 => {
                let prefix = &line_content[..byte_offset_in_line.min(line_content.len())];
                prefix.chars().count()
            }
        };

        Ok(Position::new(line as u32, character as u32))
    }

    /// Get the range of text as a Range
    pub fn range_of_text(
        &self,
        start: usize,
        end: usize,
        encoding: PositionEncoding,
    ) -> Result<Range> {
        let start_pos = self.offset_to_position(start, encoding)?;
        let end_pos = self.offset_to_position(end, encoding)?;
        Ok(Range::new(start_pos, end_pos))
    }

    /// Compute the byte offsets of line starts
    fn compute_line_starts(content: &str) -> Vec<usize> {
        let mut line_starts = vec![0];
        for (i, byte) in content.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(i + 1);
            }
        }
        line_starts
    }

    /// Get the number of lines in the document
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::Position;

    #[test]
    fn test_document_creation() {
        let content = "hello\nworld\nfoo";
        let doc = TextDocument::new(content.to_string(), 1);
        assert_eq!(doc.content(), content);
        assert_eq!(doc.version(), 1);
        assert_eq!(doc.line_count(), 3);
    }

    #[test]
    fn test_line_starts() {
        let content = "hello\nworld\nfoo";
        let doc = TextDocument::new(content.to_string(), 1);
        assert_eq!(doc.line_starts, vec![0, 6, 12]);
    }

    #[test]
    fn test_position_to_offset() {
        let content = "hello\nworld\n";
        let doc = TextDocument::new(content.to_string(), 1);

        // Start of document
        assert_eq!(
            doc.position_to_offset(Position::new(0, 0), PositionEncoding::UTF8)
                .unwrap(),
            0
        );

        // End of first line
        assert_eq!(
            doc.position_to_offset(Position::new(0, 5), PositionEncoding::UTF8)
                .unwrap(),
            5
        );

        // Start of second line
        assert_eq!(
            doc.position_to_offset(Position::new(1, 0), PositionEncoding::UTF8)
                .unwrap(),
            6
        );

        // End of second line
        assert_eq!(
            doc.position_to_offset(Position::new(1, 5), PositionEncoding::UTF8)
                .unwrap(),
            11
        );
    }

    #[test]
    fn test_offset_to_position() {
        let content = "hello\nworld\n";
        let doc = TextDocument::new(content.to_string(), 1);

        assert_eq!(
            doc.offset_to_position(0, PositionEncoding::UTF8).unwrap(),
            Position::new(0, 0)
        );

        assert_eq!(
            doc.offset_to_position(5, PositionEncoding::UTF8).unwrap(),
            Position::new(0, 5)
        );

        assert_eq!(
            doc.offset_to_position(6, PositionEncoding::UTF8).unwrap(),
            Position::new(1, 0)
        );
    }

    #[test]
    fn test_apply_incremental_change() {
        let mut doc = TextDocument::new("hello world".to_string(), 1);

        let changes = vec![TextDocumentContentChangeEvent {
            range: Some(Range::new(Position::new(0, 0), Position::new(0, 5))),
            range_length: Some(5),
            text: "hi".to_string(),
        }];

        doc.apply_changes(changes, 2, PositionEncoding::UTF8)
            .unwrap();
        assert_eq!(doc.content(), "hi world");
        assert_eq!(doc.version(), 2);
    }

    #[test]
    fn test_unicode_utf16_encoding() {
        let content = "hello 🌍 world"; // 🌍 is 2 UTF-16 code units
        let doc = TextDocument::new(content.to_string(), 1);

        // Position after the emoji in UTF-16 coordinates
        let pos = Position::new(0, 8); // "hello " (6) + "🌍" (2) = 8 UTF-16 code units
        let offset = doc
            .position_to_offset(pos, PositionEncoding::UTF16)
            .unwrap();

        // Should be at byte position 10 ("hello 🌍" in bytes)
        assert_eq!(offset, 10);

        // Converting back should give us the same position
        let back_pos = doc
            .offset_to_position(offset, PositionEncoding::UTF16)
            .unwrap();
        assert_eq!(back_pos, pos);
    }
}
