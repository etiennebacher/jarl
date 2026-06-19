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

/// Error for unknown rule names in the configuration (CLI or TOML).
///
/// Carries the main error message plus optional "did you mean" help lines,
/// which the binary renders on separate `Help:` lines.
#[derive(Debug)]
pub struct UnknownRulesError {
    pub message: String,
    pub help: Vec<String>,
}

impl fmt::Display for UnknownRulesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for UnknownRulesError {}
