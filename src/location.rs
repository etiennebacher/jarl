use serde::{Deserialize, Serialize};

/// Sourcecode location.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Location {
    pub(super) row: usize,
    pub(super) column: usize,
}

impl Location {
    pub fn fmt_with(
        &self,
        f: &mut std::fmt::Formatter,
        e: &impl std::fmt::Display,
    ) -> std::fmt::Result {
        write!(f, "{} at line {} column {}", e, self.row(), self.column())
    }

    pub fn new(row: usize, column: usize) -> Self {
        Location { row, column }
    }

    /// Current row
    pub fn row(&self) -> usize {
        self.row
    }

    /// Current column
    pub fn column(&self) -> usize {
        self.column
    }
}
