use std::fmt;
use std::path::PathBuf;

use crate::location::Location;
use biome_rowan::TextRange;
use colored::Colorize;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Fix {
    pub content: String,
    pub start: usize,
    pub end: usize,
}

impl Fix {
    pub fn empty() -> Self {
        Self {
            content: "".to_string(),
            start: 0usize,
            end: 0usize,
        }
    }
}

/// Details on the violated rule.
pub trait Violation {
    /// Name of the rule.
    fn name(&self) -> String;
    /// Explanation of the rule.
    fn body(&self) -> String;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DiagnosticKind {
    pub name: String,
    pub body: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Diagnostic {
    pub message: DiagnosticKind,
    pub filename: PathBuf,
    pub range: TextRange,
    pub location: Option<Location>,
    pub fix: Fix,
}

impl<T> From<T> for DiagnosticKind
where
    T: Violation,
{
    fn from(value: T) -> Self {
        Self {
            name: Violation::name(&value),
            body: Violation::body(&value),
        }
    }
}

impl Diagnostic {
    pub fn new<T: Into<DiagnosticKind>>(
        message: T,
        filename: &str,
        range: TextRange,
        fix: Fix,
    ) -> Self {
        Self {
            message: message.into(),
            filename: filename.into(),
            range,
            location: None,
            fix,
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (row, col) = match self.location {
            Some(loc) => (loc.row, loc.column),
            None => (0, 0),
        };
        write!(
            f,
            "{} [{}:{}] {} {}",
            self.filename.to_string_lossy().white(),
            row,
            col,
            self.message.name.red(),
            self.message.body
        )
    }
}
