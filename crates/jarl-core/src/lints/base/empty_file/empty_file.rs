use crate::diagnostic::{Diagnostic, Fix, Violation};
use air_r_syntax::RSyntaxNode;
use biome_rowan::TextRange;

pub struct EmptyFile;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Reports R files that contain no code: either truly empty, only whitespace,
/// or only comments.
///
/// ## Why is this bad?
///
/// An empty or comment-only file is almost always a mistake: a placeholder that
/// was forgotten, an accidental `touch`, or a leftover from a refactor. It adds
/// noise to the package and can confuse readers.
///
/// ## Example
///
/// ```r
/// # TODO: implement the data loader
/// ```
///
/// Use instead: delete the file, or add the intended code.
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

pub fn empty_file(expressions: &[RSyntaxNode]) -> Option<Diagnostic> {
    if !expressions.is_empty() {
        return None;
    }

    Some(Diagnostic::new(
        EmptyFile,
        TextRange::default(),
        Fix::empty(),
    ))
}
