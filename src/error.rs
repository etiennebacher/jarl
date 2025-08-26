use std::fmt;
use std::path::PathBuf;

/// Custom error type for R parsing errors.
#[derive(Debug)]
pub struct ParseError {
    pub filename: PathBuf,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Failed to parse {} due to syntax errors.",
            self.filename.display()
        )
    }
}

impl std::error::Error for ParseError {}
