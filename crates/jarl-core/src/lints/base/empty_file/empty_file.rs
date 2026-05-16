use crate::diagnostic::{Diagnostic, Fix, Violation};
use biome_rowan::TextRange;

pub struct EmptyFile;

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

pub fn empty_file() -> Diagnostic {
    Diagnostic::new(EmptyFile, TextRange::default(), Fix::empty())
}
