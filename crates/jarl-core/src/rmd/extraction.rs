//! Extraction of R code chunks from R Markdown and Quarto documents.

use regex::Regex;
use std::sync::LazyLock;

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
