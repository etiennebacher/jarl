use annotate_snippets::{Level, Renderer, Snippet};
use biome_rowan::TextRange;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::PathBuf;

use crate::location::Location;
use crate::rule_set::{FixStatus, Rule};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
// The fix to apply to the violation.
pub struct Fix {
    pub content: String,
    pub start: usize,
    pub end: usize,
    // TODO: This is used only to not add a Fix when the node contains a comment
    // because I don't know how to handle them for now, #95.
    pub to_skip: bool,
}

impl Fix {
    pub fn empty() -> Self {
        Self {
            content: "".to_string(),
            start: 0usize,
            end: 0usize,
            to_skip: true,
        }
    }
}

/// Details on the violated rule.
pub trait Violation {
    /// Name of the rule.
    fn name(&self) -> String;
    /// Explanation of the rule.
    fn body(&self) -> String;
    /// Optional suggestion for how to fix the violation.
    fn suggestion(&self) -> Option<String> {
        None
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ViolationData {
    pub name: String,
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
            name: Violation::name(&value),
            body: Violation::body(&value),
            suggestion: Violation::suggestion(&value),
        }
    }
}

impl ViolationData {
    pub fn new(name: String, body: String, suggestion: Option<String>) -> Self {
        Self { name, body, suggestion }
    }

    pub fn empty() -> Self {
        Self {
            name: "".to_string(),
            body: "".to_string(),
            suggestion: None,
        }
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

    pub fn empty() -> Self {
        Self {
            message: ViolationData::empty(),
            range: TextRange::empty(0.into()),
            location: None,
            fix: Fix::empty(),
            filename: "".into(),
        }
    }

    // TODO: in these three functions, the first condition should be removed
    // once comments in nodes are better handled, #95.
    pub fn has_safe_fix(&self) -> bool {
        if self.fix.to_skip || self.fix.content.is_empty() {
            return false;
        }
        Rule::from_name(&self.message.name)
            .map(|r| r.fix_status() == FixStatus::Safe)
            .unwrap_or(false)
    }
    pub fn has_unsafe_fix(&self) -> bool {
        if self.fix.to_skip || self.fix.content.is_empty() {
            return false;
        }
        Rule::from_name(&self.message.name)
            .map(|r| r.fix_status() == FixStatus::Unsafe)
            .unwrap_or(false)
    }
    pub fn has_no_fix(&self) -> bool {
        if self.fix.to_skip {
            return true;
        }
        Rule::from_name(&self.message.name)
            .map(|r| r.fix_status() == FixStatus::None)
            .unwrap_or(true)
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
pub fn render_diagnostic(
    source: &str,
    origin: &str,
    title: &str,
    diagnostic: &Diagnostic,
    renderer: &Renderer,
) -> String {
    let start_offset: usize = diagnostic.range.start().into();
    let end_offset: usize = diagnostic.range.end().into();

    let snippet = Snippet::source(source)
        .origin(origin)
        .fold(true)
        .annotation(
            Level::Warning
                .span(start_offset..end_offset)
                .label(&diagnostic.message.body),
        );

    let mut message = Level::Warning.title(title).snippet(snippet);

    if let Some(suggestion_text) = &diagnostic.message.suggestion {
        message = message.footer(Level::Help.title(suggestion_text));
    }

    format!("{}", renderer.render(message))
}
