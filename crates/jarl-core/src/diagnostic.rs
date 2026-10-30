use annotate_snippets::{AnnotationKind, Level, Padding, Renderer, Snippet};
use biome_rowan::{TextRange, TextSize};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::PathBuf;

use crate::location::Location;
use crate::rule_set::{FixStatus, Rule};
use crate::utils::{line_start, next_line_start};

/// A single contiguous replacement: the source covered by `range` becomes
/// `content`.
///
/// An empty `range` is an insertion at that offset, and an empty `content` is a
/// deletion.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Edit {
    // Portion of the source replaced by `content`.
    pub range: TextRange,
    pub content: String,
}

impl Edit {
    /// Replace the source covered by `range` with `content`.
    pub fn replacement(range: TextRange, content: String) -> Self {
        Self { range, content }
    }

    /// Remove the source covered by `range`.
    pub fn deletion(range: TextRange) -> Self {
        Self { range, content: String::new() }
    }

    /// Same as [`Edit::deletion`] for callers that compute byte offsets outside
    /// of the syntax tree (e.g. from the raw source).
    pub fn deletion_with_offsets(start: usize, end: usize) -> Self {
        Self::deletion(TextRange::new(
            TextSize::from(start as u32),
            TextSize::from(end as u32),
        ))
    }

    /// Remove the whole line containing `offset`, line break included.
    pub fn delete_line(source: &str, offset: usize) -> Self {
        Self::deletion_with_offsets(line_start(source, offset), next_line_start(source, offset))
    }

    /// Insert `content` at `at`, leaving the surrounding source untouched.
    pub fn insertion(at: TextSize, content: String) -> Self {
        Self { range: TextRange::new(at, at), content }
    }

    pub fn start(&self) -> usize {
        self.range.start().into()
    }

    pub fn end(&self) -> usize {
        self.range.end().into()
    }

    /// An edit that replaces nothing with nothing leaves the source unchanged.
    fn is_noop(&self) -> bool {
        self.range.is_empty() && self.content.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
// The fix to apply to the violation.
pub struct Fix {
    /// Edits applied together, all-or-nothing, sorted by start offset.
    ///
    /// Most fixes hold a single edit, but a repair that touches disjoint spots
    /// (removing both comments of a `jarl-ignore-start`/`jarl-ignore-end` pair,
    /// say) holds one edit per spot.
    pub edits: Vec<Edit>,
    // TODO: This is used only to not add a Fix when the node contains a comment
    // because I don't know how to handle them for now, #95.
    pub to_skip: bool,
}

impl Fix {
    /// Replace the source covered by `range` with `content`.
    pub fn new(range: TextRange, content: String, to_skip: bool) -> Self {
        Self::from_edits(vec![Edit::replacement(range, content)], to_skip)
    }

    /// Same as [`Fix::new`] for callers that compute byte offsets outside
    /// of the syntax tree (e.g. from the raw source).
    pub fn new_with_offsets(start: usize, end: usize, content: String, to_skip: bool) -> Self {
        Self::new(
            TextRange::new(TextSize::from(start as u32), TextSize::from(end as u32)),
            content,
            to_skip,
        )
    }

    /// Build a fix out of several edits applied together.
    ///
    /// Edits that change nothing are dropped, and the rest are sorted by start
    /// offset. Edits of a single fix must not overlap each other: they describe
    /// one repair, so an overlap is a bug in the rule rather than something to
    /// resolve at apply time.
    pub fn from_edits(edits: Vec<Edit>, to_skip: bool) -> Self {
        let mut edits: Vec<Edit> = edits.into_iter().filter(|e| !e.is_noop()).collect();
        edits.sort_by_key(|e| (e.range.start(), e.range.end()));
        Self { edits, to_skip }
    }

    pub fn empty() -> Self {
        Self { edits: Vec::new(), to_skip: true }
    }

    /// Start of the first edit, or `0` when the fix changes nothing.
    pub fn start(&self) -> usize {
        self.edits.first().map_or(0, Edit::start)
    }

    /// End of the last edit, or `0` when the fix changes nothing.
    pub fn end(&self) -> usize {
        self.edits.last().map_or(0, Edit::end)
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

/// An extra annotated range shown alongside the diagnostic's own range, to
/// point at the code that explains the violation (the earlier assignment a
/// later one overwrites, say).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub range: TextRange,
    pub text: String,
    /// Row and column of the start of `range`, filled in once the diagnostic
    /// location is computed.
    #[serde(default)]
    pub location: Option<Location>,
}

impl Label {
    pub fn new(range: TextRange, text: String) -> Self {
        Self { range, text, location: None }
    }
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
    /// The rule had a fix but it was dropped because the violation is in a
    /// roxygen `@examples` section and `fix-roxygen` is off. Only used to tell
    /// the user why an otherwise fixable rule is reported as unfixable.
    #[serde(skip)]
    pub fix_disabled_in_roxygen: bool,
    /// Extra ranges that explain the violation. Suppression matching, fixes and
    /// the primary location of every emitter keep using `range`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<Label>,
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
            fix_disabled_in_roxygen: false,
            labels: Vec::new(),
        }
    }

    pub fn with_labels(mut self, labels: Vec<Label>) -> Self {
        self.labels = labels;
        self
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
    let range = |r: TextRange| -> std::ops::Range<usize> { r.start().into()..r.end().into() };
    let main = range(diagnostic.range);
    let spans =
        std::iter::once(main.clone()).chain(diagnostic.labels.iter().map(|l| range(l.range)));
    let (window, window_start, first_line) = snippet_window(source, spans);
    let rebase = |r: std::ops::Range<usize>| (r.start - window_start)..(r.end - window_start);

    // With no labels there is a single annotation, and `Context` is what every
    // other rule renders. With labels, the diagnostic's own range must be
    // `Primary` so that the `-->` header keeps pointing at it rather than at
    // the earliest label.
    let kind = if diagnostic.labels.is_empty() {
        AnnotationKind::Context
    } else {
        AnnotationKind::Primary
    };
    let mut snippet = Snippet::source(window)
        .path(origin)
        .line_start(first_line)
        .fold(true)
        .annotation(kind.span(rebase(main)).label(&diagnostic.message.body));
    for label in &diagnostic.labels {
        snippet = snippet.annotation(
            AnnotationKind::Context
                .span(rebase(range(label.range)))
                .label(&label.text),
        );
    }

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

    let (window, window_start, first_line) =
        snippet_window(source, std::iter::once(start_offset..end_offset));

    let snippet = Snippet::source(window)
        .path(origin)
        .line_start(first_line)
        .fold(true)
        .annotation(
            AnnotationKind::Primary
                .span((start_offset - window_start)..(end_offset - window_start)),
        );

    let group = Level::ERROR
        .primary_title(error.message.as_str())
        .element(snippet)
        .element(Padding);

    renderer.render(&[group]).to_string()
}

/// Narrow `source` down to the lines the annotations actually cover.
///
/// `annotate_snippets` indexes every line of the source it is handed, once per
/// rendered diagnostic, so passing whole files makes rendering cost scale with
/// file size times diagnostic count. Folding hides the surrounding lines but
/// does not skip that indexing, so the trimming has to happen here.
///
/// The window stretches from the first span to the last, so with several spans
/// its cost is bounded by the distance between them.
///
/// The line number is counted from `source` rather than taken from the
/// diagnostic's `location`, so the gutter always agrees with the lines actually
/// shown.
///
/// Returns the window, the byte offset of its start in `source` (to rebase the
/// spans into it), and the 1-based number of its first line.
fn snippet_window(
    source: &str,
    spans: impl Iterator<Item = std::ops::Range<usize>>,
) -> (&str, usize, usize) {
    let (start, end) = spans.fold((usize::MAX, 0), |(start, end), span| {
        (start.min(span.start), end.max(span.end))
    });

    // Find the line range covering the spans: from the newline before `start`
    // to the newline after `end`.
    let line_start = source[..start].rfind('\n').map_or(0, |p| p + 1);
    let line_end = source[end..].find('\n').map_or(source.len(), |p| end + p);

    let first_line = 1 + source.as_bytes()[..line_start]
        .iter()
        .filter(|&&b| b == b'\n')
        .count();

    (&source[line_start..line_end], line_start, first_line)
}
