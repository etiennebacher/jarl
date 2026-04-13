//! Extraction of R code chunks from R Markdown and Quarto documents.

use air_r_parser::RParserOptions;
use biome_rowan::TextRange;
use biome_rowan::TextSize;
use regex::Regex;
use std::sync::LazyLock;

use crate::directive::{
    DirectiveParseResult, LintDirective, is_quarto_chunk_array_header,
    parse_quarto_chunk_array_item,
};

/// Matches the opening fence of an executable R code chunk.
///
/// Captures group 1: the backtick sequence (e.g. "```").
/// Accepts `{r}`, `{r label}`, `{r, options}`, etc.
/// Leading spaces or tabs are allowed to support indented chunks (e.g. inside
/// list items).
static OPEN_FENCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[ \t]*(`{3,})\{[rR][^}]*\}").unwrap());

/// An R code chunk extracted from an Rmd/Qmd document.
#[derive(Debug)]
pub struct RCodeChunk {
    /// The raw source code of the chunk (without fence lines).
    pub code: String,
    /// Byte offset in the original file where the chunk code starts.
    /// This is the byte immediately after the opening fence line's newline.
    pub start_byte: usize,
}

/// Extract all executable R code chunks from Rmd/Qmd content.
///
/// Only fenced chunks whose opening line matches ` ```{r...} ` (any number of
/// backticks ≥ 3) are returned. Display-only ` ```r ` blocks and tilde-fenced
/// blocks are skipped. The closing fence must use the same number of backticks
/// as the opening fence.
pub fn extract_r_chunks(content: &str) -> Vec<RCodeChunk> {
    let mut chunks = Vec::new();
    let mut byte_offset: usize = 0;

    // State: None = outside a chunk, Some((fence, code, start_byte)) = inside.
    let mut current: Option<(String, String, usize)> = None;

    for line in content.split_inclusive('\n') {
        let mut finished = false;

        if let Some((fence, code, start_byte)) = current.as_mut() {
            if line.trim() == fence.as_str() {
                // Closing fence found — emit the chunk.
                chunks.push(RCodeChunk {
                    code: std::mem::take(code),
                    start_byte: *start_byte,
                });
                finished = true;
            } else {
                code.push_str(line);
            }
        } else if let Some(caps) = OPEN_FENCE.captures(line) {
            // Opening fence found — start a new chunk.
            let fence = caps.get(1).unwrap().as_str().to_string();
            // The chunk code starts immediately after this line.
            let chunk_start_byte = byte_offset + line.len();
            current = Some((fence, String::new(), chunk_start_byte));
        }

        if finished {
            current = None;
        }

        byte_offset += line.len();
    }

    chunks
}

/// A segment mapping virtual-string byte positions to original-file byte positions.
#[derive(Debug, Clone)]
struct Segment {
    /// Start byte in the virtual R string.
    virtual_start: usize,
    /// Length in the virtual R string.
    virtual_len: usize,
    /// Corresponding start byte in the original Rmd file.
    original_start: usize,
    /// Length in the original file (may differ from `virtual_len` for translated lines).
    original_len: usize,
}

/// Maps byte offsets from a virtual concatenated R string back to the original
/// Rmd/Qmd file positions.
#[derive(Debug, Clone)]
pub struct OffsetMap {
    segments: Vec<Segment>,
}

impl OffsetMap {
    /// Remap a single byte offset from virtual-string space to original-file space.
    fn remap_offset(&self, offset: usize) -> usize {
        // Binary search for the segment containing this offset.
        let idx = self
            .segments
            .partition_point(|s| s.virtual_start + s.virtual_len <= offset);
        if idx < self.segments.len() {
            let seg = &self.segments[idx];
            let offset_within = offset.saturating_sub(seg.virtual_start);
            seg.original_start + offset_within.min(seg.original_len.saturating_sub(1))
        } else if let Some(last) = self.segments.last() {
            // Past the end — clamp to end of last segment.
            last.original_start + last.original_len
        } else {
            offset
        }
    }

    /// Remap a `TextRange` from virtual-string space to original-file space.
    pub fn remap_range(&self, range: TextRange) -> TextRange {
        let start: usize = range.start().into();
        let end: usize = range.end().into();
        let new_start = self.remap_offset(start);
        let new_end = self.remap_offset(end);
        TextRange::new(
            TextSize::from(new_start as u32),
            TextSize::from(new_end as u32),
        )
    }
}

/// Parsed chunk suppression info for translation.
struct ChunkIgnoreBlock {
    /// Rules with their full `rule: reason` text for start comments.
    rules: Vec<(String, String)>, // (rule_name, "rule: reason")
    /// Byte range within the chunk code covering the `#|` header + item lines.
    header_start: usize,
    header_end: usize,
    /// Per-item byte ranges within the chunk code (for offset mapping).
    item_ranges: Vec<(usize, usize)>, // (start, end) within chunk code
}

/// Scan a chunk's code for `#| jarl-ignore-chunk:` YAML blocks and collect
/// the rules and byte ranges.
fn find_chunk_ignore_blocks(code: &str) -> Vec<ChunkIgnoreBlock> {
    let mut blocks = Vec::new();
    let mut offset = 0;

    for line in code.split_inclusive('\n') {
        if is_quarto_chunk_array_header(line) {
            let header_start = offset;
            let mut rules = Vec::new();
            let mut item_ranges = Vec::new();
            let mut scan_offset = offset + line.len();

            // Look ahead for YAML array items.
            for item_line in code[scan_offset..].split_inclusive('\n') {
                match parse_quarto_chunk_array_item(item_line) {
                    Some(DirectiveParseResult::Valid(LintDirective::IgnoreChunk(rule))) => {
                        let rule_name = rule.name().to_string();
                        // Reconstruct "rule: reason" from the parsed item line.
                        let trimmed = item_line.trim();
                        let rest = trimmed.strip_prefix("#|").unwrap_or(trimmed);
                        let rest = rest.trim_start().strip_prefix('-').unwrap_or(rest);
                        let rule_reason = rest.trim().to_string();
                        rules.push((rule_name, rule_reason));
                        item_ranges.push((scan_offset, scan_offset + item_line.len()));
                        scan_offset += item_line.len();
                    }
                    Some(_) => {
                        // Invalid item (missing explanation, bad rule name) — still
                        // part of the YAML block. Include it so we skip past it, but
                        // don't add a rule. The suppression system will report it.
                        scan_offset += item_line.len();
                    }
                    None => break, // Not a YAML item — stop look-ahead.
                }
            }

            if !rules.is_empty() {
                blocks.push(ChunkIgnoreBlock {
                    rules,
                    header_start,
                    header_end: scan_offset,
                    item_ranges,
                });
            }
        }
        offset += line.len();
    }

    blocks
}

/// Build a virtual R source string by concatenating all valid R chunks,
/// translating `#| jarl-ignore-chunk:` YAML blocks into
/// `# jarl-ignore-start` / `# jarl-ignore-end` pairs.
///
/// Chunks with parse errors are silently dropped.
///
/// Returns the virtual source and an `OffsetMap` for remapping diagnostic
/// byte offsets back to the original Rmd file.
pub fn build_virtual_r_source(chunks: &[RCodeChunk]) -> (String, OffsetMap) {
    let mut virtual_src = String::new();
    let mut segments: Vec<Segment> = Vec::new();

    for chunk in chunks {
        // Skip empty chunks.
        if chunk.code.trim().is_empty() {
            continue;
        }

        // Pre-validate: skip chunks with parse errors.
        let parsed = air_r_parser::parse(&chunk.code, RParserOptions::default());
        if parsed.has_error() {
            continue;
        }

        let blocks = find_chunk_ignore_blocks(&chunk.code);

        if blocks.is_empty() {
            // No YAML ignore blocks — emit chunk code as-is.
            let v_start = virtual_src.len();
            virtual_src.push_str(&chunk.code);
            segments.push(Segment {
                virtual_start: v_start,
                virtual_len: chunk.code.len(),
                original_start: chunk.start_byte,
                original_len: chunk.code.len(),
            });
        } else {
            // Translate YAML blocks into start/end comments.
            emit_translated_chunk(
                &chunk.code,
                chunk.start_byte,
                &blocks,
                &mut virtual_src,
                &mut segments,
            );
        }

        // Ensure chunks are separated by a newline.
        if !virtual_src.ends_with('\n') {
            virtual_src.push('\n');
        }
    }

    (virtual_src, OffsetMap { segments })
}

/// Emit a single chunk with YAML ignore blocks translated to start/end comments.
fn emit_translated_chunk(
    code: &str,
    start_byte: usize,
    blocks: &[ChunkIgnoreBlock],
    virtual_src: &mut String,
    segments: &mut Vec<Segment>,
) {
    // Collect all rules from all blocks (for prepend/append).
    let all_rules: Vec<&(String, String)> = blocks.iter().flat_map(|b| &b.rules).collect();

    // Prepend `# jarl-ignore-start` lines.
    for (_rule_name, rule_reason) in &all_rules {
        let start_comment = format!("# jarl-ignore-start {rule_reason}\n");
        let v_start = virtual_src.len();
        virtual_src.push_str(&start_comment);
        // Map to the corresponding item line in the original file.
        // Find the item range for this rule.
        let item_original = find_item_original(blocks, rule_reason);
        segments.push(Segment {
            virtual_start: v_start,
            virtual_len: start_comment.len(),
            original_start: start_byte + item_original.0,
            original_len: item_original.1 - item_original.0,
        });
    }

    // Emit the chunk code, replacing YAML block lines with inert `#\n` comments.
    let mut code_offset = 0;
    for block in blocks {
        // Emit code before this block.
        if code_offset < block.header_start {
            let slice = &code[code_offset..block.header_start];
            let v_start = virtual_src.len();
            virtual_src.push_str(slice);
            segments.push(Segment {
                virtual_start: v_start,
                virtual_len: slice.len(),
                original_start: start_byte + code_offset,
                original_len: slice.len(),
            });
        }

        // Replace each line in the YAML block (header + items) with `#\n`.
        let block_text = &code[block.header_start..block.header_end];
        let mut line_offset = block.header_start;
        for line in block_text.split_inclusive('\n') {
            let replacement = "#\n";
            let v_start = virtual_src.len();
            virtual_src.push_str(replacement);
            segments.push(Segment {
                virtual_start: v_start,
                virtual_len: replacement.len(),
                original_start: start_byte + line_offset,
                original_len: line.len(),
            });
            line_offset += line.len();
        }

        code_offset = block.header_end;
    }

    // Emit remaining code after last block.
    if code_offset < code.len() {
        let slice = &code[code_offset..];
        let v_start = virtual_src.len();
        virtual_src.push_str(slice);
        segments.push(Segment {
            virtual_start: v_start,
            virtual_len: slice.len(),
            original_start: start_byte + code_offset,
            original_len: slice.len(),
        });
    }

    // Append `# jarl-ignore-end` lines.
    for (rule_name, _rule_reason) in &all_rules {
        let end_comment = format!("# jarl-ignore-end {rule_name}\n");
        let v_start = virtual_src.len();
        virtual_src.push_str(&end_comment);
        // Map to the YAML header line as fallback.
        let header_start = blocks[0].header_start;
        let header_end = code[header_start..]
            .find('\n')
            .map_or(code.len(), |p| header_start + p + 1);
        segments.push(Segment {
            virtual_start: v_start,
            virtual_len: end_comment.len(),
            original_start: start_byte + header_start,
            original_len: header_end - header_start,
        });
    }
}

/// Find the original byte range for a rule's item line in the YAML blocks.
fn find_item_original(blocks: &[ChunkIgnoreBlock], rule_reason: &str) -> (usize, usize) {
    for block in blocks {
        for (i, (_name, reason)) in block.rules.iter().enumerate() {
            if reason == rule_reason {
                return block.item_ranges[i];
            }
        }
    }
    // Fallback: return the first block's header range.
    (blocks[0].header_start, blocks[0].header_end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_extraction() {
        let content = "# Title\n\n```{r}\nx <- 1\n```\n";
        let chunks = extract_r_chunks(content);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].code, "x <- 1\n");
    }

    #[test]
    fn test_display_only_block_skipped() {
        // ```r without braces should be skipped
        let content = "```r\nx <- 1\n```\n";
        let chunks = extract_r_chunks(content);
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_four_backtick_fence() {
        let content = "````{r}\nx <- 1\n````\n";
        let chunks = extract_r_chunks(content);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].code, "x <- 1\n");
    }

    #[test]
    fn test_four_backtick_fence_not_closed_by_three() {
        // Opening with 4 backticks must be closed with 4, not 3.
        let content = "````{r}\nx <- 1\n```\nstill inside\n````\n";
        let chunks = extract_r_chunks(content);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].code, "x <- 1\n```\nstill inside\n");
    }

    #[test]
    fn test_start_byte() {
        let header = "# Title\n\n```{r}\n";
        let content = format!("{}x <- 1\n```\n", header);
        let chunks = extract_r_chunks(&content);
        assert_eq!(chunks.len(), 1);
        // start_byte should point right after the opening fence line
        assert_eq!(chunks[0].start_byte, header.len());
    }

    #[test]
    fn test_multiple_chunks() {
        let content = "```{r}\na <- 1\n```\n\n```{r}\nb <- 2\n```\n";
        let chunks = extract_r_chunks(content);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].code, "a <- 1\n");
        assert_eq!(chunks[1].code, "b <- 2\n");
    }

    #[test]
    fn test_capital_r() {
        let content = "```{R}\nx <- 1\n```\n";
        let chunks = extract_r_chunks(content);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn test_chunk_with_label_and_options() {
        let content = "```{r my-chunk, echo=FALSE}\nx <- 1\n```\n";
        let chunks = extract_r_chunks(content);
        assert_eq!(chunks.len(), 1);
    }

    // --- Edge cases ---

    #[test]
    fn test_unclosed_chunk_produces_no_output() {
        // A chunk that is never closed should be silently dropped.
        let content = "```{r}\nany(is.na(x))\n";
        let chunks = extract_r_chunks(content);
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_empty_chunk() {
        // A chunk with no code between the fences.
        let content = "```{r}\n```\n";
        let chunks = extract_r_chunks(content);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].code, "");
    }

    #[test]
    fn test_no_trailing_newline() {
        // Content that does not end with a newline character.
        let content = "```{r}\nx <- 1\n```";
        let chunks = extract_r_chunks(content);
        // Closing fence has no trailing newline, trim_end() still matches "```".
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].code, "x <- 1\n");
    }

    #[test]
    fn test_tilde_fence_skipped() {
        // Quarto/Rmd only use backtick fences; tilde fences are not supported.
        let content = "~~~{r}\nany(is.na(x))\n~~~\n";
        let chunks = extract_r_chunks(content);
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_indented_fence_matched() {
        // Leading spaces are allowed (e.g. a chunk inside a list item).
        let content = "  ```{r}\nany(is.na(x))\n  ```\n";
        let chunks = extract_r_chunks(content);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].code, "any(is.na(x))\n");
    }

    #[test]
    fn test_indented_chunk_inside_list() {
        // Realistic list-item scenario from R Markdown / Quarto.
        let content = "* hello\n\n  ```{r}\n  any(is.na(1))\n  ```\n";
        let chunks = extract_r_chunks(content);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].code, "  any(is.na(1))\n");
    }

    #[test]
    fn test_start_byte_second_chunk() {
        // Verify that start_byte for the second chunk accounts for everything before it.
        let first = "```{r}\na <- 1\n```\n";
        let separator = "\nsome prose\n\n";
        let second_fence = "```{r}\n";
        let content = format!("{first}{separator}{second_fence}b <- 2\n```\n");
        let chunks = extract_r_chunks(&content);
        assert_eq!(chunks.len(), 2);
        let expected_start = first.len() + separator.len() + second_fence.len();
        assert_eq!(chunks[1].start_byte, expected_start);
        // The byte at start_byte should be the start of the chunk code.
        assert_eq!(
            &content[chunks[1].start_byte..chunks[1].start_byte + 6],
            "b <- 2"
        );
    }
}
