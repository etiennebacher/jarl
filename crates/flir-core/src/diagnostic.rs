use biome_rowan::TextRange;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::PathBuf;

use crate::lints::{all_nofix_rules, all_safe_rules, all_unsafe_rules};
use crate::location::Location;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
// The fix to apply to the violation.
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ViolationData {
    pub name: String,
    pub body: String,
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
        }
    }
}

impl ViolationData {
    pub fn new(name: String, body: String) -> Self {
        Self { name, body }
    }
    pub fn empty() -> Self {
        Self { name: "".to_string(), body: "".to_string() }
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

    pub fn has_safe_fix(&self) -> bool {
        all_safe_rules().contains(&self.message.name)
    }

    pub fn has_unsafe_fix(&self) -> bool {
        all_unsafe_rules().contains(&self.message.name)
    }

    pub fn has_no_fix(&self) -> bool {
        all_nofix_rules().contains(&self.message.name)
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
