use crate::diagnostic::{Diagnostic, Fix, Violation};
use air_r_syntax::RSyntaxNode;
use biome_rowan::TextRange;

pub struct EmptyFile;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Reports R files that contain no code: either truly empty, only whitespace,
/// or only plain comments.
///
/// Files that contain at least one roxygen comment (a line starting with `#'`)
/// are intentionally allowed, since packages commonly use comment-only files
/// as documentation templates (e.g. files in `man-roxygen/`).
///
/// ## Why is this bad?
///
/// An empty or comment-only file is almost always a mistake: a placeholder that
/// was forgotten, or a leftover from a refactor. It adds noise to the package and
/// can confuse readers.
///
/// ## Example
///
/// ```r
/// # TODO: implement the data loader
/// ```
///
/// Instead, delete the file or add the intended code.
impl Violation for EmptyFile {
    fn name(&self) -> String {
        "empty_file".to_string()
    }
    fn body(&self) -> String {
        "This file is empty or only contains comments.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Consider deleting the file".to_string())
    }
}

pub fn empty_file(expressions: &[RSyntaxNode], syntax: &RSyntaxNode) -> Option<Diagnostic> {
    if !expressions.is_empty() {
        return None;
    }

    // Files that contain at least one roxygen comment (`#'`) are allowed:
    // they're commonly used as documentation templates (e.g. man-roxygen/).
    let text = syntax.text_with_trivia().to_string();
    if text
        .lines()
        .any(|line| line.trim_start().starts_with("#'"))
    {
        return None;
    }

    Some(Diagnostic::new(
        EmptyFile,
        TextRange::default(),
        Fix::empty(),
    ))
}
