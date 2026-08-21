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
/// Captures group 2: the chunk options, i.e. everything between `{r` and `}`.
/// Accepts `{r}`, `{r label}`, `{r, options}`, etc.
/// Leading spaces or tabs are allowed to support indented chunks (e.g. inside
/// list items).
static OPEN_FENCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[ \t]*(`{3,})\{[rR]([^}]*)\}").unwrap());

/// Matches `eval = FALSE` among the options of a chunk header.
///
/// Case-sensitive on the value: `FALSE` and `F` are R's false literals, while
/// `false` is an ordinary symbol. A value this doesn't match (`eval = run_it`,
/// `eval = nrow(x) > 0`) is decided at render time, and treating those as
/// evaluated is the conservative reading — see [`RCodeChunk::evaluated`].
static HEADER_EVAL_FALSE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\beval\s*=\s*(FALSE|F)\b").unwrap());

/// Matches Quarto's `#| eval: false` chunk option.
///
/// The value is a YAML boolean, so unlike the header form it is not
/// case-sensitive. `#| eval: !expr <code>` is resolved at render time and
/// deliberately doesn't match.
static YAML_EVAL_FALSE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^[ \t]*#\|[ \t]*eval[ \t]*:[ \t]*false[ \t]*$").unwrap());

/// Matches a Quarto chunk option whose value is R code: `#| key: !expr code`.
///
/// Captures group 1: the R code after `!expr`.
static YAML_EXPR_OPTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[ \t]*#\|[^:]*:[ \t]*!expr[ \t]+(.+?)[ \t]*$").unwrap());

/// Matches inline R code in prose: `` `r expr` ``.
///
/// Captures group 1: the R expression. The backtick-delimited span cannot
/// itself contain a backtick, which also keeps the opening fence of a chunk
/// (```` ```{r} ````) from matching — there the `r` is preceded by `{`, not by
/// whitespace.
static INLINE_CODE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"`[rR][ \t\r\n]([^`]*)`").unwrap());

/// An R code chunk extracted from an Rmd/Qmd document.
#[derive(Debug)]
pub struct RCodeChunk {
    /// The raw source code of the chunk (without fence lines).
    pub code: String,
    /// Byte offset in the original file where the chunk code starts.
    /// This is the byte immediately after the opening fence line's newline.
    pub start_byte: usize,
    /// The chunk options as written in the opening fence, i.e. everything
    /// between `{r` and `}` (`" my-chunk, echo = FALSE"`).
    pub options: String,
    /// Whether the chunk runs when the document is rendered, i.e. whether it
    /// is *not* marked `eval = FALSE` (header form) or `#| eval: false`
    /// (Quarto form).
    ///
    /// Only literal false values count. An `eval` whose value is an expression
    /// is unknowable here, and guessing "not evaluated" is the harmful
    /// direction: it would drop the chunk's reads, which can turn an object
    /// used only there into a false `unused_object` report. Guessing
    /// "evaluated" can only leave a diagnostic unreported.
    pub evaluated: bool,
}

/// A chunk being accumulated between its opening and closing fence.
struct PendingChunk {
    code: String,
    start_byte: usize,
    header_eval_false: bool,
    options: String,
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

    // State: None = outside a chunk, Some(chunk being accumulated) = inside.
    let mut current: Option<(String, PendingChunk)> = None;

    for line in content.split_inclusive('\n') {
        let mut finished = false;

        if let Some((fence, pending)) = current.as_mut() {
            if line.trim() == fence.as_str() {
                // Closing fence found — emit the chunk.
                let code = std::mem::take(&mut pending.code);
                chunks.push(RCodeChunk {
                    evaluated: !pending.header_eval_false && !yaml_says_eval_false(&code),
                    code,
                    start_byte: pending.start_byte,
                    options: std::mem::take(&mut pending.options),
                });
                finished = true;
            } else {
                pending.code.push_str(line);
            }
        } else if let Some(caps) = OPEN_FENCE.captures(line) {
            // Opening fence found — start a new chunk.
            let fence = caps.get(1).unwrap().as_str().to_string();
            let options = caps.get(2).unwrap().as_str().to_string();
            current = Some((
                fence,
                PendingChunk {
                    code: String::new(),
                    // The chunk code starts immediately after this line.
                    start_byte: byte_offset + line.len(),
                    header_eval_false: HEADER_EVAL_FALSE.is_match(&options),
                    options,
                },
            ));
        }

        if finished {
            current = None;
        }

        byte_offset += line.len();
    }

    chunks
}

/// Whether the chunk's Quarto option block sets `eval: false`.
///
/// Quarto only reads `#|` options from the run of lines at the very top of the
/// chunk, so the scan stops at the first line that isn't one. A `#| eval:
/// false` further down is a plain comment.
fn yaml_says_eval_false(code: &str) -> bool {
    code.lines()
        .take_while(|line| line.trim_start().starts_with("#|"))
        .any(|line| YAML_EVAL_FALSE.is_match(line))
}

/// The R code found in a chunk's options, as snippets the caller can parse.
///
/// Both option forms can name an object: `{r, fig.cap = my_caption}` in the
/// fence header, and `#| fig-cap: !expr my_caption` in Quarto's block. Knitr
/// evaluates a chunk's options when it renders the chunk — that is how
/// `eval = run_it` gets to decide anything — so this holds for every chunk,
/// including one whose own code never runs.
///
/// The header snippet is wrapped as a call so that it parses as R, which lets
/// the caller tell an option's *value* (`my_caption`, a read) from its *name*
/// (`fig.cap`, not a read). Header options are R's named-argument syntax
/// already, so the wrapping is all it takes. A leading chunk label is dropped:
/// it is a bare word, often not even valid R (`{r my-chunk}`).
pub fn chunk_option_code(chunk: &RCodeChunk) -> Vec<String> {
    let mut snippets = Vec::new();

    if let Some(options) = header_options_without_label(&chunk.options) {
        snippets.push(format!("list({options})"));
    }

    snippets.extend(
        chunk
            .code
            .lines()
            .take_while(|line| line.trim_start().starts_with("#|"))
            .filter_map(|line| YAML_EXPR_OPTION.captures(line))
            .map(|caps| caps.get(1).unwrap().as_str().to_string()),
    );

    snippets
}

/// The chunk options with the leading label removed, or `None` when nothing
/// but a label is left.
fn header_options_without_label(options: &str) -> Option<&str> {
    let rest = match options.split_once(',') {
        // A first segment without `=` is the label, not an option.
        Some((first, rest)) if !first.contains('=') => rest,
        // A single segment without `=` is a label on its own.
        None => return None,
        _ => options,
    };

    let rest = rest.trim();
    (!rest.is_empty()).then_some(rest)
}

/// Extract the R expressions of inline code spans (`` `r expr` ``) written in
/// the document's prose.
///
/// Two kinds of match are left out:
/// - spans inside a code chunk, where a backtick pair is part of an R string
///   (`glue("`r x`")`) rather than something the document evaluates;
/// - spans whose opening backtick is itself preceded by one, which is the
///   fence of a display-only ` ```r ` block, not an inline span.
///
/// `chunks` must be the output of [`extract_r_chunks`] for the same content,
/// whose spans are ordered and non-overlapping.
pub fn extract_inline_r_code<'a>(content: &'a str, chunks: &[RCodeChunk]) -> Vec<&'a str> {
    INLINE_CODE
        .captures_iter(content)
        .filter(|caps| {
            let start = caps.get(0).unwrap().start();
            !in_a_chunk(start, chunks) && !content[..start].ends_with('`')
        })
        .map(|caps| caps.get(1).unwrap().as_str())
        .collect()
}

/// Whether `offset` falls within the code of one of `chunks`.
fn in_a_chunk(offset: usize, chunks: &[RCodeChunk]) -> bool {
    let idx = chunks.partition_point(|c| c.start_byte + c.code.len() <= offset);
    chunks.get(idx).is_some_and(|c| offset >= c.start_byte)
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

/// An invalid YAML array item that should be translated into a comment
/// recognizable by the suppression system.
struct InvalidChunkItem {
    /// The original `rule: reason` text extracted from the YAML item.
    text: String,
    /// The parse result (`InvalidRuleName`, `MissingExplanation`, etc.).
    result: DirectiveParseResult,
    /// Byte range within the chunk code.
    range: (usize, usize),
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
    /// Invalid items that should be emitted as detectable comments.
    invalid_items: Vec<InvalidChunkItem>,
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
            let mut invalid_items = Vec::new();
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
                    Some(result) => {
                        // Invalid item (missing explanation, bad rule name) — still
                        // part of the YAML block. Track it so we can emit a
                        // detectable comment in the virtual source.
                        let trimmed = item_line.trim();
                        let rest = trimmed.strip_prefix("#|").unwrap_or(trimmed);
                        let rest = rest.trim_start().strip_prefix('-').unwrap_or(rest);
                        let text = rest.trim().to_string();
                        invalid_items.push(InvalidChunkItem {
                            text,
                            result,
                            range: (scan_offset, scan_offset + item_line.len()),
                        });
                        scan_offset += item_line.len();
                    }
                    None => break, // Not a YAML item — stop look-ahead.
                }
            }

            if !rules.is_empty() || !invalid_items.is_empty() {
                blocks.push(ChunkIgnoreBlock {
                    rules,
                    header_start,
                    header_end: scan_offset,
                    item_ranges,
                    invalid_items,
                });
            }
        }
        offset += line.len();
    }

    blocks
}

/// The concatenated R code of a document, plus what it took to build it.
pub struct VirtualSource {
    /// The virtual R source: every valid chunk's code, in document order.
    pub source: String,
    /// Maps offsets in `source` back to the original Rmd/Qmd file.
    pub offset_map: OffsetMap,
    /// Indices into the input chunks of the chunks left out because they have
    /// parse errors. Their code is absent from `source`, so anything they read
    /// is invisible to the analysis unless the caller accounts for it.
    pub skipped: Vec<usize>,
    /// Spans in `source` covering the chunks that don't run when the document
    /// is rendered (see [`RCodeChunk::evaluated`]). Their code is still linted
    /// — it is code the author wrote and readers see — but it neither defines
    /// nor reads anything, so use-def analysis has to leave it out.
    pub unevaluated: Vec<TextRange>,
}

/// Build a virtual R source string by concatenating all valid R chunks,
/// translating `#| jarl-ignore-chunk:` YAML blocks into
/// `# jarl-ignore-start` / `# jarl-ignore-end` pairs.
///
/// Chunks with parse errors are dropped, and reported in
/// [`VirtualSource::skipped`].
pub fn build_virtual_r_source(chunks: &[RCodeChunk]) -> VirtualSource {
    let mut virtual_src = String::new();
    let mut segments: Vec<Segment> = Vec::new();
    let mut skipped: Vec<usize> = Vec::new();
    let mut unevaluated: Vec<TextRange> = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        // Skip empty chunks.
        if chunk.code.trim().is_empty() {
            continue;
        }

        // Pre-validate: skip chunks with parse errors.
        let parsed = air_r_parser::parse(&chunk.code, RParserOptions::default());
        if parsed.has_error() {
            skipped.push(i);
            continue;
        }

        let chunk_start = virtual_src.len();
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

        if !chunk.evaluated {
            unevaluated.push(TextRange::new(
                TextSize::from(chunk_start as u32),
                TextSize::from(virtual_src.len() as u32),
            ));
        }
    }

    VirtualSource {
        source: virtual_src,
        offset_map: OffsetMap { segments },
        skipped,
        unevaluated,
    }
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

        // Replace each line in the YAML block with either an inert `#\n`
        // comment or a detectable `# jarl-ignore` comment for invalid items.
        let block_text = &code[block.header_start..block.header_end];
        let mut line_offset = block.header_start;
        for line in block_text.split_inclusive('\n') {
            let line_start = line_offset;
            let line_end = line_offset + line.len();

            // Check if this line corresponds to an invalid item.
            let replacement =
                if let Some(item) = block.invalid_items.iter().find(|i| i.range.0 == line_start) {
                    match item.result {
                        DirectiveParseResult::InvalidRuleName => {
                            format!("# jarl-ignore {}\n", item.text)
                        }
                        DirectiveParseResult::MissingExplanation => {
                            format!("# jarl-ignore {}\n", item.text)
                        }
                        _ => "#\n".to_string(),
                    }
                } else {
                    "#\n".to_string()
                };

            let v_start = virtual_src.len();
            virtual_src.push_str(&replacement);
            segments.push(Segment {
                virtual_start: v_start,
                virtual_len: replacement.len(),
                original_start: start_byte + line_start,
                original_len: line_end - line_start,
            });
            line_offset = line_end;
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

    // --- Inline R code ---

    /// Extract the inline spans of `content`, in document order.
    fn inline(content: &str) -> Vec<&str> {
        let chunks = extract_r_chunks(content);
        extract_inline_r_code(content, &chunks)
    }

    #[test]
    fn test_inline_code_in_prose() {
        assert_eq!(inline("The mean is `r mean(x)`.\n"), vec!["mean(x)"]);
    }

    #[test]
    fn test_several_inline_spans_on_one_line() {
        assert_eq!(inline("`r a` and `r b`\n"), vec!["a", "b"]);
    }

    #[test]
    fn test_inline_code_spanning_lines() {
        assert_eq!(inline("Value: `r mean(\n  x\n)`\n"), vec!["mean(\n  x\n)"]);
    }

    #[test]
    fn test_capital_r_inline_code() {
        assert_eq!(inline("`R x`\n"), vec!["x"]);
    }

    #[test]
    fn test_inline_code_inside_a_chunk_ignored() {
        // A backtick pair in chunk code is an R string, not an inline span.
        let content = "```{r}\nglue(\"`r x`\")\n```\n";
        assert!(inline(content).is_empty());
    }

    #[test]
    fn test_inline_code_around_a_chunk() {
        // Spans before and after a chunk are kept; the fences themselves and
        // the chunk body are not spans.
        let content = "`r before`\n\n```{r}\ny <- 1\n```\n\n`r after`\n";
        assert_eq!(inline(content), vec!["before", "after"]);
    }

    #[test]
    fn test_chunk_fence_is_not_inline_code() {
        assert!(inline("```{r}\nx <- 1\n```\n").is_empty());
    }

    #[test]
    fn test_display_only_r_block_is_not_inline_code() {
        // ```r opens a display block, so its fences are not an inline span.
        assert!(inline("```r\nx\n```\n").is_empty());
    }

    // --- eval = FALSE ---

    /// Whether the single chunk of `content` is marked as evaluated.
    fn evaluated(content: &str) -> bool {
        let chunks = extract_r_chunks(content);
        assert_eq!(chunks.len(), 1, "expected exactly one chunk");
        chunks[0].evaluated
    }

    #[test]
    fn test_chunk_without_eval_option_is_evaluated() {
        assert!(evaluated("```{r}\nx <- 1\n```\n"));
        assert!(evaluated("```{r label, echo=FALSE}\nx <- 1\n```\n"));
    }

    #[test]
    fn test_header_eval_false_is_not_evaluated() {
        assert!(!evaluated("```{r, eval = FALSE}\nx <- 1\n```\n"));
        assert!(!evaluated("```{r eval=FALSE}\nx <- 1\n```\n"));
        assert!(!evaluated("```{r label, eval=F, echo=TRUE}\nx <- 1\n```\n"));
    }

    #[test]
    fn test_header_eval_true_is_evaluated() {
        assert!(evaluated("```{r, eval = TRUE}\nx <- 1\n```\n"));
        assert!(evaluated("```{r, eval=T}\nx <- 1\n```\n"));
    }

    #[test]
    fn test_header_eval_expression_is_evaluated() {
        // Decided at render time. Assuming it runs can only cost a diagnostic;
        // assuming it doesn't could invent one.
        assert!(evaluated("```{r, eval = run_it}\nx <- 1\n```\n"));
        assert!(evaluated("```{r, eval = nrow(d) > 0}\nx <- 1\n```\n"));
    }

    #[test]
    fn test_eval_is_not_confused_with_another_option() {
        // A different option whose name ends in `eval`, and a value that
        // merely mentions FALSE.
        assert!(evaluated("```{r, reeval = FALSE}\nx <- 1\n```\n"));
        assert!(evaluated("```{r, echo = FALSE}\nx <- 1\n```\n"));
    }

    #[test]
    fn test_quarto_eval_false_is_not_evaluated() {
        assert!(!evaluated("```{r}\n#| eval: false\nx <- 1\n```\n"));
        assert!(!evaluated(
            "```{r}\n#| label: a\n#| eval: FALSE\nx <- 1\n```\n"
        ));
    }

    #[test]
    fn test_quarto_eval_true_is_evaluated() {
        assert!(evaluated("```{r}\n#| eval: true\nx <- 1\n```\n"));
    }

    #[test]
    fn test_quarto_eval_expression_is_evaluated() {
        assert!(evaluated("```{r}\n#| eval: !expr run_it\nx <- 1\n```\n"));
    }

    #[test]
    fn test_quarto_option_below_the_code_is_a_comment() {
        // Quarto only reads the `#|` block at the top of the chunk.
        assert!(evaluated("```{r}\nx <- 1\n#| eval: false\n```\n"));
    }

    #[test]
    fn test_unevaluated_chunk_span_covers_its_code() {
        let content = "```{r}\nkeep <- 1\n```\n\n```{r, eval = FALSE}\ndead <- 2\n```\n";
        let chunks = extract_r_chunks(content);
        let virtual_source = build_virtual_r_source(&chunks);

        assert_eq!(virtual_source.source, "keep <- 1\ndead <- 2\n");
        assert_eq!(virtual_source.unevaluated.len(), 1);
        let range = virtual_source.unevaluated[0];
        assert_eq!(&virtual_source.source[range], "dead <- 2\n");
    }

    #[test]
    fn test_evaluated_chunks_have_no_unevaluated_span() {
        let chunks = extract_r_chunks("```{r}\nx <- 1\n```\n");
        assert!(build_virtual_r_source(&chunks).unevaluated.is_empty());
    }

    // --- Skipped chunks ---

    #[test]
    fn test_chunk_with_parse_error_is_reported_as_skipped() {
        let content = "```{r}\nx <- 1\n```\n\n```{r}\nif (y\n```\n";
        let chunks = extract_r_chunks(content);
        let virtual_source = build_virtual_r_source(&chunks);
        assert_eq!(virtual_source.skipped, vec![1]);
        assert_eq!(virtual_source.source, "x <- 1\n");
    }

    #[test]
    fn test_valid_chunks_are_not_reported_as_skipped() {
        let chunks = extract_r_chunks("```{r}\nx <- 1\n```\n\n```{r}\n```\n");
        assert!(build_virtual_r_source(&chunks).skipped.is_empty());
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

    // --- Chunk options ---

    /// The option snippets of the single chunk in `content`.
    fn option_code(content: &str) -> Vec<String> {
        let chunks = extract_r_chunks(content);
        assert_eq!(chunks.len(), 1, "expected exactly one chunk");
        chunk_option_code(&chunks[0])
    }

    #[test]
    fn test_header_options_are_wrapped_as_a_call() {
        assert_eq!(
            option_code("```{r, fig.cap = my_caption}\nx <- 1\n```\n"),
            vec!["list(fig.cap = my_caption)"]
        );
    }

    #[test]
    fn test_header_label_is_dropped() {
        // A label is a bare word, and `my-chunk` isn't even valid R.
        assert_eq!(
            option_code("```{r my-chunk, eval = run_it}\nx <- 1\n```\n"),
            vec!["list(eval = run_it)"]
        );
    }

    #[test]
    fn test_header_with_only_a_label_has_no_option_code() {
        assert!(option_code("```{r my-chunk}\nx <- 1\n```\n").is_empty());
        assert!(option_code("```{r}\nx <- 1\n```\n").is_empty());
    }

    #[test]
    fn test_header_option_value_may_contain_a_comma() {
        assert_eq!(
            option_code("```{r, fig.cap = paste(a, b)}\nx <- 1\n```\n"),
            vec!["list(fig.cap = paste(a, b))"]
        );
    }

    #[test]
    fn test_quarto_expr_option_is_r_code() {
        assert_eq!(
            option_code("```{r}\n#| eval: !expr run_it\nx <- 1\n```\n"),
            vec!["run_it"]
        );
    }

    #[test]
    fn test_quarto_plain_option_is_not_r_code() {
        // Without `!expr` the value is data, not code.
        assert!(option_code("```{r}\n#| fig-cap: my_caption\nx <- 1\n```\n").is_empty());
    }

    #[test]
    fn test_quarto_expr_option_below_the_code_is_ignored() {
        assert!(option_code("```{r}\nx <- 1\n#| eval: !expr run_it\n```\n").is_empty());
    }

    #[test]
    fn test_both_option_forms_are_collected() {
        assert_eq!(
            option_code("```{r, fig.cap = cap}\n#| eval: !expr run_it\nx <- 1\n```\n"),
            vec!["list(fig.cap = cap)", "run_it"]
        );
    }
}
