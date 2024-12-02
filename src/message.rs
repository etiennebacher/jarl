use std::fmt;
use std::path::PathBuf;

use colored::Colorize;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Location {
    pub row: usize,
    pub column: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    TrueFalseSymbol {
        filename: PathBuf,
        location: Location,
    },
}

impl Message {
    /// Short ID for the message.
    pub fn code(&self) -> &'static str {
        match self {
            Message::TrueFalseSymbol { filename: _, location: _ } => "T-F-symbols",
        }
    }

    /// The body text for the message.
    pub fn body(&self) -> &'static str {
        match self {
            Message::TrueFalseSymbol { filename: _, location: _ } => "`T` and `F` can be confused with variable names. Spell `TRUE` and `FALSE` entirely instead.",
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Message::TrueFalseSymbol { filename, location } => write!(
                f,
                "{}{}{}{}{}  {}\t{}",
                filename.to_string_lossy().white().bold(),
                ":  ".cyan(),
                location.column,
                ":".cyan(),
                location.row,
                self.code().red().bold(),
                self.body()
            ),
        }
    }
}
