use std::fmt;
use std::path::PathBuf;

use crate::location::Location;
use colored::Colorize;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DiagnosticKind {
    pub name: String,
    pub body: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Diagnostic {
    pub message: DiagnosticKind,
    pub filename: PathBuf,
    pub location: Location,
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

impl DiagnosticKind {
    pub fn empty() -> Self {
        Self { name: "".to_string(), body: "".to_string() }
    }
}

impl Diagnostic {
    pub fn new<T: Into<DiagnosticKind>>(
        message: T,
        filename: &str,
        location: Location,
        fix: Fix,
    ) -> Self {
        Self {
            message: message.into(),
            filename: filename.into(),
            location,
            fix,
        }
    }

    pub fn empty() -> Self {
        Self {
            message: DiagnosticKind::empty(),
            filename: "".to_string().into(),
            location: Location::new(0, 0),
            fix: Fix::empty(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} [{}:{}] {} {}",
            self.filename.to_string_lossy().white(),
            self.location.row,
            self.location.column,
            self.message.name.red(),
            self.message.body
        )
    }
}
