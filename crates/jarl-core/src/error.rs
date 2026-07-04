use std::fmt;
use std::path::PathBuf;

use crate::diagnostic::Diagnostic;

/// Custom error type for R parsing errors.
///
/// The parser recovers from syntax errors, so the rest of the file is still
/// linted: the diagnostics found in the code that parsed successfully are
/// carried here for the caller to report alongside the error.
#[derive(Debug)]
pub struct ParseError {
    pub filename: PathBuf,
    pub diagnostics: Vec<Diagnostic>,
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
