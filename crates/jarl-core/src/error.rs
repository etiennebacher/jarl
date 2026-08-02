use std::fmt;
use std::path::PathBuf;

use biome_rowan::TextRange;

use crate::diagnostic::Diagnostic;
use crate::location::Location;

/// A single syntax error reported by the parser.
///
/// Carries the parser's message (e.g. `expected an expression`) along with the
/// range it points at and its resolved (row, column) location, so the caller
/// can render it as an annotated snippet.
#[derive(Debug)]
pub struct SyntaxError {
    pub message: String,
    pub range: TextRange,
    pub location: Location,
}

/// Custom error type for R parsing errors.
///
/// The parser recovers from syntax errors, so the rest of the file is still
/// linted: the diagnostics found in the code that parsed successfully are
/// carried here for the caller to report alongside the error. The individual
/// syntax errors (message + location) are carried in `syntax_errors` so the
/// caller can report each of them precisely rather than a single generic line.
#[derive(Debug)]
pub struct ParseError {
    pub filename: PathBuf,
    pub diagnostics: Vec<Diagnostic>,
    pub syntax_errors: Vec<SyntaxError>,
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
