use crate::diagnostic::*;

pub struct EmptyFile;

impl Violation for EmptyFile {
    fn name(&self) -> String {
        "empty_file".to_string()
    }
    fn body(&self) -> String {
        "This file is empty.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Consider deleting the file".to_string())
    }
}
