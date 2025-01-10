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
    pub applied: bool,
    pub length_change: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LintData {
    pub filename: PathBuf,
    pub location: Location,
    pub fix: Fix,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    TrueFalseSymbol(LintData),
    AnyIsNa(LintData),
    AnyDuplicated(LintData),
}

impl Message {
    pub fn filename(&self) -> &PathBuf {
        match self {
            Message::TrueFalseSymbol(data) => &data.filename,
            Message::AnyIsNa(data) => &data.filename,
            Message::AnyDuplicated(data) => &data.filename,
        }
    }

    pub fn location(&self) -> &Location {
        match self {
            Message::TrueFalseSymbol(data) => &data.location,
            Message::AnyIsNa(data) => &data.location,
            Message::AnyDuplicated(data) => &data.location,
        }
    }

    pub fn fix(&self) -> &Fix {
        match self {
            Message::TrueFalseSymbol(data) => &data.fix,
            Message::AnyIsNa(data) => &data.fix,
            Message::AnyDuplicated(data) => &data.fix,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Message::TrueFalseSymbol { .. } => "T-F-symbols",
            Message::AnyIsNa { .. } => "any-na",
            Message::AnyDuplicated { .. } => "any-duplicated",
        }
    }
    pub fn body(&self) -> &'static str {
        match self {
            Message::TrueFalseSymbol { .. } => "`T` and `F` can be confused with variable names. Spell `TRUE` and `FALSE` entirely instead.",
            Message::AnyIsNa { .. } => "`any(is.na(...))` is inefficient. Use `anyNA(...)` instead.",
            Message::AnyDuplicated { .. } => "`any(duplicated(...))` is inefficient. Use `anyDuplicated(...) > 0` instead.",
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Message::AnyDuplicated(_) | Message::AnyIsNa(_) | Message::TrueFalseSymbol(_) => {
                write!(
                    f,
                    "{} [{}:{}] {} {}",
                    self.filename().to_string_lossy().white().bold(),
                    self.location().row,
                    self.location().column,
                    self.code().red().bold(),
                    self.body()
                )
            }
        }
    }
}
