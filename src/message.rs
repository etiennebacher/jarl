use std::fmt;
use std::path::PathBuf;

use crate::location::Location;
use colored::Colorize;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Fix {
    pub content: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    TrueFalseSymbol {
        filename: PathBuf,
        location: Location,
        fix: Fix,
    },
    AnyIsNa {
        filename: PathBuf,
        location: Location,
        fix: Fix,
    },
    AnyDuplicated {
        filename: PathBuf,
        location: Location,
        fix: Fix,
    },
    ClassEquals {
        filename: PathBuf,
        location: Location,
        fix: Fix,
    },
    EqualsNa {
        filename: PathBuf,
        location: Location,
        fix: Fix,
    },
}

impl Message {
    pub fn code(&self) -> &'static str {
        match self {
            Message::TrueFalseSymbol { .. } => "T-F-symbols",
            Message::AnyIsNa { .. } => "any-na",
            Message::AnyDuplicated { .. } => "any-duplicated",
            Message::ClassEquals { .. } => "class-equals",
            Message::EqualsNa { .. } => "equals-na",
        }
    }
    pub fn body(&self) -> &'static str {
        match self {
            Message::TrueFalseSymbol { .. } => "`T` and `F` can be confused with variable names. Spell `TRUE` and `FALSE` entirely instead.",
            Message::AnyIsNa { .. } => "`any(is.na(...))` is inefficient. Use `anyNA(...)` instead.",
            Message::AnyDuplicated { .. } => "`any(duplicated(...))` is inefficient. Use `anyDuplicated(...) > 0` instead.",
            Message::ClassEquals { .. } => "Use `inherits(..., 'x')` instead of `class(...) == 'x'.`",
            Message::EqualsNa { .. } => "Use `is.na()` instead of comparing to NA with ==, != or %in%.",

        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Message::AnyDuplicated { filename, location, .. }
            | Message::AnyIsNa { filename, location, .. }
            | Message::ClassEquals { filename, location, .. }
            | Message::EqualsNa { filename, location, .. }
            | Message::TrueFalseSymbol { filename, location, .. } => write!(
                f,
                "{} [{}:{}] {} {}",
                filename.to_string_lossy().white().bold(),
                location.row,
                location.column,
                self.code().red().bold(),
                self.body()
            ),
        }
    }
}
