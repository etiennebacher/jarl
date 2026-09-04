use annotate_snippets::{AnnotationKind, Level, Padding, Renderer, Snippet};
use biome_rowan::{TextRange, TextSize};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::PathBuf;

use crate::location::Location;
use crate::rule_set::{FixStatus, Rule};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
// The fix to apply to the violation.
pub struct Fix {
    pub content: String,
    // Portion of the source replaced by `content`.
    pub range: TextRange,
    // TODO: This is used only to not add a Fix when the node contains a comment
    // because I don't know how to handle them for now, #95.
    pub to_skip: bool,
}

impl Fix {
    /// Replace the source covered by `range` with `content`.
    pub fn new(range: TextRange, content: String, to_skip: bool) -> Self {
        Self { content, range, to_skip }
    }

    /// Same as [`Fix::replace`] for callers that compute byte offsets outside
    /// of the syntax tree (e.g. from the raw source).
    pub fn new_with_offsets(start: usize, end: usize, content: String, to_skip: bool) -> Self {
        Self::new(
            TextRange::new(TextSize::from(start as u32), TextSize::from(end as u32)),
            content,
            to_skip,
        )
    }

    pub fn empty() -> Self {
        Self {
            content: "".to_string(),
            range: TextRange::default(),
            to_skip: true,
        }
    }

    pub fn start(&self) -> usize {
        self.range.start().into()
    }

    pub fn end(&self) -> usize {
        self.range.end().into()
    }
}

/// Details on the violated rule.
pub trait Violation {
    /// The violated rule.
    fn rule(&self) -> Rule;
    /// Explanation of the rule.
    fn body(&self) -> String;
    /// Optional suggestion for how to fix the violation.
    fn suggestion(&self) -> Option<String> {
        None
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ViolationData {
    // Serialized as "name" so the JSON output keeps the key it always had.
    #[serde(rename = "name")]
    pub rule: Rule,
    pub body: String,
    pub suggestion: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
// The object that is eventually reported and printed in the console.
pub struct Diagnostic {
    // The name and description of the violated rule.
    pub message: ViolationData,
    // Location of the violated rule.
    pub filename: PathBuf,
    pub range: TextRange,
    pub location: Option<Location>,
    // Fix to apply if the user passed `--fix`.
    pub fix: Fix,
}

impl<T: Violation> From<T> for ViolationData {
    fn from(value: T) -> Self {
        Self {
            rule: Violation::rule(&value),
            body: Violation::body(&value),
            suggestion: Violation::suggestion(&value),
        }
    }
}

impl ViolationData {
    pub fn new(rule: Rule, body: String, suggestion: Option<String>) -> Self {
        Self { rule, body, suggestion }
    }
}

impl Diagnostic {
    pub fn new<T: Into<ViolationData>>(message: T, range: TextRange, fix: Fix) -> Self {
        Self {
            message: message.into(),
            range,
            location: None,
            fix,
            filename: "".into(),
        }
    }

    // TODO: in these three functions, the first condition should be removed
    // once comments in nodes are better handled, #95.
    pub fn has_safe_fix(&self) -> bool {
        !self.fix.to_skip && self.message.rule.fix_status() == FixStatus::Safe
    }
    pub fn has_unsafe_fix(&self) -> bool {
        !self.fix.to_skip && self.message.rule.fix_status() == FixStatus::Unsafe
    }
    pub fn has_no_fix(&self) -> bool {
        self.fix.to_skip || self.message.rule.fix_status() == FixStatus::None
    }
}

impl Ord for Diagnostic {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare first by filename, then by range
        match self.filename.cmp(&other.filename) {
            Ordering::Equal => self.range.cmp(&other.range),
            other => other,
        }
    }
}

impl PartialOrd for Diagnostic {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Render a single diagnostic as an annotated code snippet.
///
/// Uses `annotate_snippets` to produce a formatted message with the source
/// context, warning label, and optional suggestion footer.
///
/// The `title` parameter allows callers to customize the message title
/// (e.g. the CLI uses a hyperlinked rule name, while tests use the plain name).
/// It is passed as a `secondary_title` because that is the only title
/// constructor that leaves the text untouched: `primary_title` normalizes
/// control characters, which would mangle the OSC 8 escapes of the hyperlink.
pub fn render_diagnostic(
    source: &str,
    origin: &str,
    title: &str,
    diagnostic: &Diagnostic,
    renderer: &Renderer,
) -> String {
    let start_offset: usize = diagnostic.range.start().into();
    let end_offset: usize = diagnostic.range.end().into();

    let (window, adj_start, adj_end, first_line) = snippet_window(source, start_offset, end_offset);

    let snippet = Snippet::source(window)
        .path(origin)
        .line_start(first_line)
        .fold(true)
        .annotation(
            AnnotationKind::Context
                .span(adj_start..adj_end)
                .label(&diagnostic.message.body),
        );

    let mut group = Level::WARNING.secondary_title(title).element(snippet);

    // Close the snippet with a blank gutter line. When a suggestion follows,
    // the renderer already separates it from the snippet.
    if let Some(suggestion_text) = &diagnostic.message.suggestion {
        group = group.element(Level::HELP.message(suggestion_text.as_str()));
    } else {
        group = group.element(Padding);
    }

    renderer.render(&[group]).to_string()
}

/// Render a single syntax error as an annotated code snippet.
///
/// Mirrors [`render_diagnostic`] but for parser errors: the message becomes the
/// error title and the offending range is underlined in its source context.
pub fn render_syntax_error(
    source: &str,
    origin: &str,
    error: &crate::error::SyntaxError,
    renderer: &Renderer,
) -> String {
    let start_offset: usize = error.range.start().into();
    let end_offset: usize = error.range.end().into();

    let (window, adj_start, adj_end, first_line) = snippet_window(source, start_offset, end_offset);

    let snippet = Snippet::source(window)
        .path(origin)
        .line_start(first_line)
        .fold(true)
        .annotation(AnnotationKind::Primary.span(adj_start..adj_end));

    let group = Level::ERROR
        .primary_title(error.message.as_str())
        .element(snippet)
        .element(Padding);

    renderer.render(&[group]).to_string()
}

/// Narrow `source` down to the lines the annotation actually covers.
///
/// `annotate_snippets` indexes every line of the source it is handed, once per
/// rendered diagnostic, so passing whole files makes rendering cost scale with
/// file size times diagnostic count. Folding hides the surrounding lines but
/// does not skip that indexing, so the trimming has to happen here.
///
/// The line number is counted from `source` rather than taken from the
/// diagnostic's `location`, so the gutter always agrees with the lines actually
/// shown.
///
/// Returns the window, the span rebased into it, and the 1-based number of the
/// window's first line.
fn snippet_window(source: &str, start: usize, end: usize) -> (&str, usize, usize, usize) {
    // Find the line range covering the span: from the newline before `start`
    // to the newline after `end`.
    let line_start = source[..start].rfind('\n').map_or(0, |p| p + 1);
    let line_end = source[end..].find('\n').map_or(source.len(), |p| end + p);

    let first_line = 1 + source.as_bytes()[..line_start]
        .iter()
        .filter(|&&b| b == b'\n')
        .count();

    (
        &source[line_start..line_end],
        start - line_start,
        end - line_start,
        first_line,
    )
}
