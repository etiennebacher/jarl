use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Print diagnostics in a concise format, one per line
    #[default]
    Concise,
    /// Print diagnostics as JSON
    Json,
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Concise => write!(f, "concise"),
            Self::Json => write!(f, "json"),
        }
    }
}
