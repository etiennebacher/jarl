//! Extraction of R code from roxygen `@examples` and `@examplesIf` sections.
//!
//! Walks the parsed CST to find comment trivia tokens that form roxygen blocks
//! (lines starting with `#'`), locates `@examples` / `@examplesIf` tags within
//! those blocks, and extracts the subsequent R code lines with their `#' `
//! prefix stripped.

use air_r_syntax::{RLanguage, RSyntaxNode};
use biome_rowan::{SyntaxNode, TextSize};
use regex::Regex;
use std::sync::LazyLock;

/// Matches a roxygen comment prefix: one or more `#` followed by `'`.
static RE_ROXYGEN_PREFIX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^#+'").unwrap());

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
}

/// A single roxygen comment line with its text and position in the original file.
struct RoxygenLine {
    /// The full text of the comment token (e.g. `#' some text`).
    text: String,
    /// Byte offset of this comment token in the original file.
    start_byte: usize,
}

/// Extract all `@examples` / `@examplesIf` code chunks from a parsed R file.
///
/// Walks all trivia tokens in the CST looking for roxygen comment lines (`#'`),
/// groups them into contiguous blocks, finds `@examples` or `@examplesIf` tags,
/// and returns the code lines that follow those tags (with `#'` stripped).
pub fn extract_roxygen_examples(syntax: &RSyntaxNode) -> Vec<RoxygenExamplesChunk> {
    let roxygen_blocks = collect_roxygen_blocks(syntax);

    let mut chunks = Vec::new();
    for block in roxygen_blocks {
        chunks.extend(extract_examples_from_block(&block));
    }
    chunks
}

/// Collect contiguous groups of roxygen comment lines from the CST.
///
/// A roxygen block is a sequence of consecutive comment trivia tokens where
/// each starts with `#'` (possibly with multiple `#`s).
fn collect_roxygen_blocks(syntax: &RSyntaxNode) -> Vec<Vec<RoxygenLine>> {
    let mut blocks: Vec<Vec<RoxygenLine>> = Vec::new();
    let mut current_block: Vec<RoxygenLine> = Vec::new();

    // Walk all tokens in document order and inspect their leading trivia
    // for comment pieces.
    let raw: &SyntaxNode<RLanguage> = syntax;
    for token in raw.descendants_tokens(biome_rowan::Direction::Next) {
        for piece in token.leading_trivia().pieces() {
            if !piece.is_comments() {
                // A non-comment trivia piece (whitespace/newline) between comment
                // lines is expected, but if there's an actual code token between
                // two comment groups, that ends the block.
                continue;
            }

            let text = piece.text().to_string();
            if is_roxygen_comment(&text) {
                let start_byte: usize = piece.text_range().start().into();
                current_block.push(RoxygenLine { text, start_byte });
            } else {
                // Non-roxygen comment breaks the block
                if !current_block.is_empty() {
                    blocks.push(std::mem::take(&mut current_block));
                }
            }
        }

        // A real (non-trivia) token ends the current roxygen block, unless the
        // block will be continued by the next token's leading trivia. However,
        // in the R CST, a roxygen block before a function definition will have
        // all its comments as leading trivia of the first real token. So we
        // only break the block when we encounter a non-trivia token that does
        // NOT have roxygen comments in its leading trivia (handled naturally by
        // the loop above — each token's trivia is processed in order).
        //
        // Actually, in biome_rowan, all leading comments of a node are attached
        // to its first token's leading trivia. So roxygen blocks will appear
        // as a contiguous sequence of comment pieces in one token's trivia.
        // We flush the block after processing each token's trivia.
        if !current_block.is_empty() {
            blocks.push(std::mem::take(&mut current_block));
        }
    }

    if !current_block.is_empty() {
        blocks.push(current_block);
    }

    blocks
}

/// Given a roxygen block, find `@examples` / `@examplesIf` sections and extract
/// the R code lines that follow them.
fn extract_examples_from_block(block: &[RoxygenLine]) -> Vec<RoxygenExamplesChunk> {
    let mut chunks = Vec::new();
    let mut i = 0;

    while i < block.len() {
        let stripped = strip_roxygen_prefix(&block[i].text);
        let trimmed = stripped.trim_start();

        if trimmed.starts_with("@examples") || trimmed.starts_with("@examplesIf") {
            // For @examplesIf, the condition is on the same line and is valid R,
            // but we skip it since linting the condition alone isn't useful.
            // Collect code lines starting from the next line.
            i += 1;

            let mut code_lines: Vec<String> = Vec::new();
            let mut line_start_offsets: Vec<usize> = Vec::new();
            let mut line_prefix_lengths: Vec<usize> = Vec::new();

            while i < block.len() {
                let line_stripped = strip_roxygen_prefix(&block[i].text);
                let line_trimmed = line_stripped.trim_start();

                // A new `@tag` ends the examples section
                if line_trimmed.starts_with('@') {
                    break;
                }

                let prefix_len = block[i].text.len() - line_stripped.len();
                code_lines.push(line_stripped.to_string());
                line_start_offsets.push(block[i].start_byte);
                line_prefix_lengths.push(prefix_len);

                i += 1;
            }

            // Skip empty examples sections
            if code_lines.iter().all(|l| l.trim().is_empty()) {
                continue;
            }

            let code = code_lines.join("\n");
            chunks.push(RoxygenExamplesChunk { code, line_start_offsets, line_prefix_lengths });
        } else {
            i += 1;
        }
    }

    chunks
}

/// Check if a comment token is a roxygen comment (starts with one or more `#`
/// followed by `'`).
fn is_roxygen_comment(text: &str) -> bool {
    RE_ROXYGEN_PREFIX.is_match(text)
}

/// Strip the roxygen prefix (`#+'` plus at most one leading space) from a comment line.
///
/// Returns the remainder of the line after the prefix. Matches the same logic
/// as ark's `find_roxygen_examples_range`: strip `#+'` then at most one space,
/// to preserve intentional indentation.
fn strip_roxygen_prefix(text: &str) -> &str {
    let after_prefix = RE_ROXYGEN_PREFIX
        .find(text)
        .map_or(text, |m| &text[m.end()..]);
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

/// Map a byte offset within the chunk's `code` string to the corresponding byte
/// offset in the original file.
fn remap_byte_offset(offset: usize, chunk: &RoxygenExamplesChunk) -> usize {
    // Find which line of the chunk this offset falls on.
    // Lines in `code` are joined by `\n`, so line boundaries are at cumulative
    // lengths + 1 (for the newline separator).
    let mut cumulative = 0usize;
    let lines: Vec<&str> = chunk.code.split('\n').collect();

    for (line_idx, line) in lines.iter().enumerate() {
        let line_end = cumulative + line.len();

        if offset <= line_end || line_idx == lines.len() - 1 {
            // The offset falls on this line
            let col = offset.saturating_sub(cumulative);
            let original_line_start = chunk.line_start_offsets[line_idx];
            let prefix_len = chunk.line_prefix_lengths[line_idx];
            return original_line_start + prefix_len + col;
        }

        // +1 for the \n separator between lines in `code`
        cumulative = line_end + 1;
    }

    // Fallback (shouldn't happen)
    offset
}

#[cfg(test)]
mod tests {
    use super::*;
    use air_r_parser::RParserOptions;

    fn parse_and_extract(source: &str) -> Vec<RoxygenExamplesChunk> {
        let parsed = air_r_parser::parse(source, RParserOptions::default());
        extract_roxygen_examples(&parsed.syntax())
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
}
