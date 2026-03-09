//! Extraction of R code from roxygen `@examples` and `@examplesIf` sections.
//!
//! Walks the parsed CST to find comment trivia tokens that form roxygen blocks
//! (lines starting with `#'`), locates `@examples` / `@examplesIf` tags within
//! those blocks, and extracts the subsequent R code lines with their `#' `
//! prefix stripped.

use crate::diagnostic::Fix;
use air_r_syntax::{RLanguage, RSyntaxNode};
use biome_rowan::{SyntaxNode, TextSize};

/// An R code chunk extracted from a roxygen `@examples` or `@examplesIf` section.
#[derive(Debug)]
pub struct RoxygenExamplesChunk {
    /// The extracted R code with `#'` prefixes stripped.
    pub code: String,
    /// For each line in `code`, the byte offset in the original file where the
    /// original `#'` comment line starts.
    pub line_start_offsets: Vec<usize>,
    /// For each line in `code`, the number of bytes stripped from the beginning
    /// of the original comment line (the `#' ` prefix length). Used to remap
    /// column positions back to the original file.
    pub line_prefix_lengths: Vec<usize>,
    /// Pre-computed byte offset of each line within `code` (cumulative lengths
    /// accounting for `\n` separators). Used by `remap_byte_offset` to avoid
    /// repeated string splitting.
    code_line_starts: Vec<usize>,
}

/// Extract all `@examples` / `@examplesIf` code chunks from a parsed R file.
///
/// Walks all trivia tokens in the CST looking for roxygen comment lines (`#'`),
/// finds `@examples` or `@examplesIf` tags, and returns the code lines that
/// follow those tags (with `#'` stripped). Extraction and filtering happen in a
/// single pass to avoid intermediate allocations for non-examples lines.
pub fn extract_roxygen_examples(syntax: &RSyntaxNode, contents: &str) -> Vec<RoxygenExamplesChunk> {
    // Fast path: skip the CST walk if the file has no roxygen examples at all
    if !contents.contains("#'") || !contents.contains("@examples") {
        return Vec::new();
    }

    let mut chunks = Vec::new();

    // State machine for single-pass extraction:
    // - `in_examples`: we are collecting code lines after an @examples tag
    // - `in_block`: we are inside a contiguous roxygen block
    let mut in_examples = false;
    let mut in_block = false;
    let mut code_lines: Vec<String> = Vec::new();
    let mut line_start_offsets: Vec<usize> = Vec::new();
    let mut line_prefix_lengths: Vec<usize> = Vec::new();

    let raw: &SyntaxNode<RLanguage> = syntax;
    for token in raw.descendants_tokens(biome_rowan::Direction::Next) {
        for piece in token.leading_trivia().pieces() {
            if !piece.is_comments() {
                continue;
            }

            let text = piece.text();
            if is_roxygen_comment(text) {
                in_block = true;
                let stripped = strip_roxygen_prefix(text);
                let trimmed = stripped.trim_start();

                if trimmed.starts_with("@examples") || trimmed.starts_with("@examplesIf") {
                    // Flush any previous examples section in this block
                    flush_chunk(
                        &mut chunks,
                        &mut code_lines,
                        &mut line_start_offsets,
                        &mut line_prefix_lengths,
                    );
                    in_examples = true;
                } else if trimmed.starts_with('@') {
                    // A different @tag ends the examples section
                    flush_chunk(
                        &mut chunks,
                        &mut code_lines,
                        &mut line_start_offsets,
                        &mut line_prefix_lengths,
                    );
                    in_examples = false;
                } else if in_examples {
                    let prefix_len = text.len() - stripped.len();
                    let start_byte: usize = piece.text_range().start().into();
                    code_lines.push(stripped.to_string());
                    line_start_offsets.push(start_byte);
                    line_prefix_lengths.push(prefix_len);
                }
            } else {
                // Non-roxygen comment breaks the block
                if in_block {
                    flush_chunk(
                        &mut chunks,
                        &mut code_lines,
                        &mut line_start_offsets,
                        &mut line_prefix_lengths,
                    );
                    in_examples = false;
                    in_block = false;
                }
            }
        }

        // End of token's trivia — flush the roxygen block
        if in_block {
            flush_chunk(
                &mut chunks,
                &mut code_lines,
                &mut line_start_offsets,
                &mut line_prefix_lengths,
            );
            in_examples = false;
            in_block = false;
        }
    }

    // Flush any trailing examples
    flush_chunk(
        &mut chunks,
        &mut code_lines,
        &mut line_start_offsets,
        &mut line_prefix_lengths,
    );

    chunks
}

/// Flush accumulated examples lines into a chunk (if non-empty).
fn flush_chunk(
    chunks: &mut Vec<RoxygenExamplesChunk>,
    code_lines: &mut Vec<String>,
    line_start_offsets: &mut Vec<usize>,
    line_prefix_lengths: &mut Vec<usize>,
) {
    if code_lines.is_empty() {
        return;
    }

    // Strip \dontrun{}, \donttest{}, \dontshow{} wrappers
    strip_roxygen_macros(code_lines, line_start_offsets, line_prefix_lengths);

    // Skip empty examples sections
    if !code_lines.iter().all(|l| l.trim().is_empty()) {
        let code = code_lines.join("\n");
        let code_line_starts = compute_code_line_starts(&code);
        chunks.push(RoxygenExamplesChunk {
            code,
            line_start_offsets: std::mem::take(line_start_offsets),
            line_prefix_lengths: std::mem::take(line_prefix_lengths),
            code_line_starts,
        });
    }

    code_lines.clear();
    line_start_offsets.clear();
    line_prefix_lengths.clear();
}

/// Remove `\dontrun{}`, `\donttest{}`, and `\dontshow{}` wrapper lines from
/// the extracted code lines. These are roxygen macros whose content is valid R
/// code. We remove the opening `\dontrun{` line and its matching closing `}`
/// line so the code inside can be parsed and linted.
fn strip_roxygen_macros(
    code_lines: &mut Vec<String>,
    line_start_offsets: &mut Vec<usize>,
    line_prefix_lengths: &mut Vec<usize>,
) {
    // Work backwards so that removing lines doesn't shift indices we haven't
    // processed yet. Track macro nesting depth to match closing braces.
    let mut to_remove: Vec<usize> = Vec::new();
    let mut open_stack: Vec<usize> = Vec::new();

    for (i, line) in code_lines.iter().enumerate() {
        let trimmed = line.trim();
        if is_roxygen_macro(trimmed) {
            open_stack.push(i);
        } else if trimmed == "}" && !open_stack.is_empty() {
            to_remove.push(open_stack.pop().unwrap());
            to_remove.push(i);
        }
    }

    // Remove in reverse order to preserve indices
    to_remove.sort_unstable();
    to_remove.dedup();
    for &idx in to_remove.iter().rev() {
        code_lines.remove(idx);
        line_start_offsets.remove(idx);
        line_prefix_lengths.remove(idx);
    }
}

/// Check if a trimmed line is a roxygen macro like `\dontrun{`, `\donttest{`,
/// or `\dontshow{` (with optional trailing whitespace).
fn is_roxygen_macro(trimmed: &str) -> bool {
    let rest = if let Some(r) = trimmed.strip_prefix("\\dontrun{") {
        r
    } else if let Some(r) = trimmed.strip_prefix("\\donttest{") {
        r
    } else if let Some(r) = trimmed.strip_prefix("\\dontshow{") {
        r
    } else {
        return false;
    };
    rest.trim().is_empty()
}

/// Check if a comment token is a roxygen comment (starts with one or more `#`
/// followed by `'`).
fn is_roxygen_comment(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i] == b'#' {
        i += 1;
    }
    i > 0 && i < bytes.len() && bytes[i] == b'\''
}

/// Strip the roxygen prefix (`#+'` plus at most one leading space) from a comment line.
///
/// Returns the remainder of the line after the prefix. Matches the same logic
/// as ark's `find_roxygen_examples_range`: strip `#+'` then at most one space,
/// to preserve intentional indentation.
fn strip_roxygen_prefix(text: &str) -> &str {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i] == b'#' {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'\'' {
        i += 1;
    }
    let after_prefix = &text[i..];
    after_prefix.strip_prefix(' ').unwrap_or(after_prefix)
}

/// Remap a byte range from a roxygen examples chunk back to the original file.
///
/// `chunk_range` is a `TextRange` within the chunk's `code` string.
/// Returns the corresponding `TextRange` in the original file.
pub fn remap_roxygen_range(
    chunk_range: biome_rowan::TextRange,
    chunk: &RoxygenExamplesChunk,
) -> biome_rowan::TextRange {
    let start: usize = chunk_range.start().into();
    let end: usize = chunk_range.end().into();

    let new_start = remap_byte_offset(start, chunk);
    let new_end = remap_byte_offset(end, chunk);

    biome_rowan::TextRange::new(
        TextSize::from(new_start as u32),
        TextSize::from(new_end as u32),
    )
}

/// Remap a fix from chunk-local byte offsets to original file positions.
///
/// This remaps `fix.start` and `fix.end`, and also inserts the roxygen comment
/// prefix (e.g. `#' `) before each new line in the fix content so that the
/// replacement text is valid roxygen when applied to the original file.
pub fn remap_roxygen_fix(fix: &Fix, chunk: &RoxygenExamplesChunk, contents: &str) -> Fix {
    let new_start = remap_byte_offset(fix.start, chunk);
    let new_end = remap_byte_offset(fix.end, chunk);

    // Determine the roxygen prefix from the line where the fix starts.
    let start_line_idx = match chunk.code_line_starts.binary_search(&fix.start) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };
    let prefix_offset = chunk.line_start_offsets[start_line_idx];
    let prefix_len = chunk.line_prefix_lengths[start_line_idx];
    let prefix = &contents[prefix_offset..prefix_offset + prefix_len];

    // Insert the roxygen prefix before each new line in the fix content.
    let content = fix.content.replace('\n', &format!("\n{prefix}"));

    Fix {
        content,
        start: new_start,
        end: new_end,
        to_skip: fix.to_skip,
    }
}

/// Pre-compute the byte offset of each line within a `code` string (lines
/// joined by `\n`).
fn compute_code_line_starts(code: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in code.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// Map a byte offset within the chunk's `code` string to the corresponding byte
/// offset in the original file.
fn remap_byte_offset(offset: usize, chunk: &RoxygenExamplesChunk) -> usize {
    // Binary search to find which line this offset falls on.
    let line_idx = match chunk.code_line_starts.binary_search(&offset) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };

    let col = offset.saturating_sub(chunk.code_line_starts[line_idx]);
    let original_line_start = chunk.line_start_offsets[line_idx];
    let prefix_len = chunk.line_prefix_lengths[line_idx];
    original_line_start + prefix_len + col
}

#[cfg(test)]
mod tests {
    use super::*;
    use air_r_parser::RParserOptions;

    fn parse_and_extract(source: &str) -> Vec<RoxygenExamplesChunk> {
        let parsed = air_r_parser::parse(source, RParserOptions::default());
        extract_roxygen_examples(&parsed.syntax(), source)
    }

    #[test]
    fn test_basic_examples_extraction() {
        let source = r#"#' Title
#' @param x A value
#' @examples
#' x <- 1
#' y <- 2
foo <- function(x) x
"#;
        let chunks = parse_and_extract(source);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].code, "x <- 1\ny <- 2");
        assert_eq!(chunks[0].line_start_offsets.len(), 2);
        assert_eq!(chunks[0].line_prefix_lengths.len(), 2);
    }

    #[test]
    fn test_examples_if_extraction() {
        let source = r#"#' Title
#' @examplesIf interactive()
#' x <- 1
foo <- function(x) x
"#;
        let chunks = parse_and_extract(source);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].code, "x <- 1");
    }

    #[test]
    fn test_examples_ends_at_next_tag() {
        let source = r#"#' Title
#' @examples
#' x <- 1
#' @returns NULL
foo <- function(x) x
"#;
        let chunks = parse_and_extract(source);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].code, "x <- 1");
    }

    #[test]
    fn test_no_examples() {
        let source = r#"#' Title
#' @param x A value
foo <- function(x) x
"#;
        let chunks = parse_and_extract(source);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_empty_examples() {
        let source = r#"#' Title
#' @examples
#' @returns NULL
foo <- function(x) x
"#;
        let chunks = parse_and_extract(source);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_multiple_roxygen_blocks() {
        let source = r#"#' First function
#' @examples
#' x <- 1
foo <- function(x) x

#' Second function
#' @examples
#' y <- 2
bar <- function(y) y
"#;
        let chunks = parse_and_extract(source);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].code, "x <- 1");
        assert_eq!(chunks[1].code, "y <- 2");
    }

    #[test]
    fn test_prefix_length_tracking() {
        // Standard `#' ` prefix is 3 bytes
        let source = "#' @examples\n#' x <- 1\nfoo <- function(x) x\n";
        let chunks = parse_and_extract(source);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].line_prefix_lengths[0], 3); // "#' " = 3
    }

    #[test]
    fn test_remap_byte_offset() {
        let source = "#' @examples\n#' x <- 1\nfoo <- function(x) x\n";
        let chunks = parse_and_extract(source);
        assert_eq!(chunks.len(), 1);

        // "x <- 1" starts at offset 0 in chunk code
        // In original file: "#' x <- 1" starts at byte 13, prefix "#' " is 3 bytes
        // So 'x' in original file is at byte 16
        let remapped = remap_byte_offset(0, &chunks[0]);
        assert_eq!(remapped, 16);
    }

    #[test]
    fn test_double_hash_is_roxygen() {
        // `##'` is also a valid roxygen comment
        let source = "##' @examples\n##' x <- 1\nfoo <- function(x) x\n";
        let chunks = parse_and_extract(source);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].code, "x <- 1");
        // "##' " prefix is 4 bytes
        assert_eq!(chunks[0].line_prefix_lengths[0], 4);
    }

    #[test]
    fn test_preserves_indentation() {
        // Only one space after `#'` is stripped
        let source = "#' @examples\n#'   indented_code()\nfoo <- function(x) x\n";
        let chunks = parse_and_extract(source);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].code, "  indented_code()");
    }

    #[test]
    fn test_dontrun_stripped() {
        let source = "\
#' @examples
#' x <- 1
#' \\dontrun{
#' y <- 2
#' }
#' z <- 3
foo <- function(x) x
";
        let chunks = parse_and_extract(source);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].code, "x <- 1\ny <- 2\nz <- 3");
    }

    #[test]
    fn test_donttest_stripped() {
        let source = "\
#' @examples
#' \\donttest{
#' any(is.na(x))
#' }
foo <- function(x) x
";
        let chunks = parse_and_extract(source);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].code, "any(is.na(x))");
    }

    #[test]
    fn test_nested_dontrun_donttest() {
        let source = "\
#' @examples
#' \\donttest{
#' \\dontrun{
#' x <- 1
#' }
#' }
foo <- function(x) x
";
        let chunks = parse_and_extract(source);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].code, "x <- 1");
    }

    #[test]
    fn test_dontshow_stripped() {
        let source = "\
#' @examples
#' \\dontshow{
#' x <- 1
#' }
foo <- function(x) x
";
        let chunks = parse_and_extract(source);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].code, "x <- 1");
    }
}
