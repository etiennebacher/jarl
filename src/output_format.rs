use air_fs::relativize_path;
use clap::ValueEnum;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json;
use std::io::Write;

use crate::message::Diagnostic;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Print diagnostics in a concise format, one per line
    #[default]
    Concise,
    /// Print diagnostics as JSON
    Json,
}

/// Takes the diagnostics and parsing errors in each file and then displays
/// them in different ways depending on the `--output-format` provided by the
/// user.
pub trait Emitter {
    fn emit<W: Write>(
        &self,
        writer: &mut W,
        diagnostics: &Vec<&Diagnostic>,
        errors: &Vec<(String, anyhow::Error)>,
    ) -> anyhow::Result<()>;
}

pub struct ConciseEmitter;

impl Emitter for ConciseEmitter {
    fn emit<W: Write>(
        &self,
        writer: &mut W,
        diagnostics: &Vec<&Diagnostic>,
        errors: &Vec<(String, anyhow::Error)>,
    ) -> anyhow::Result<()> {
        let mut total_diagnostics = 0;
        let mut n_diagnostic_with_fixes = 0usize;
        let mut n_diagnostic_with_unsafe_fixes = 0usize;

        // First, print all parsing errors
        if !errors.is_empty() {
            for (_path, err) in errors {
                let root_cause = err.chain().last().unwrap();
                if root_cause.is::<crate::error::ParseError>() {
                    eprintln!("{}: {}", "Error".red().bold(), root_cause);
                } else {
                    eprintln!("{}: {}", "Error".red().bold(), err);
                }
            }
        }

        // Then, print the diagnostics.
        for diagnostic in diagnostics {
            let (row, col) = match diagnostic.location {
                Some(loc) => (loc.row, loc.column),
                None => {
                    unreachable!("Row/col locations must have been parsed successfully before.")
                }
            };
            write!(
                writer,
                "{} [{}:{}] {} {}\n",
                relativize_path(diagnostic.filename.clone()).white(),
                row,
                col,
                diagnostic.message.name.red(),
                diagnostic.message.body
            )?;

            if diagnostic.has_safe_fix() {
                n_diagnostic_with_fixes += 1;
            }
            if diagnostic.has_unsafe_fix() {
                n_diagnostic_with_unsafe_fixes += 1;
            }
            total_diagnostics += 1;
        }

        // Finally, print the info about the number of errors found and how
        // many can be fixed.
        if total_diagnostics > 0 {
            if total_diagnostics > 1 {
                println!("\nFound {} errors.", total_diagnostics);
            } else {
                println!("\nFound 1 error.");
            }

            if n_diagnostic_with_fixes > 0 {
                let msg = if n_diagnostic_with_unsafe_fixes == 0 {
                    format!("{n_diagnostic_with_fixes} fixable with the `--fix` option.")
                } else {
                    let unsafe_label = if n_diagnostic_with_unsafe_fixes == 1 {
                        "1 hidden fix".to_string()
                    } else {
                        format!("{n_diagnostic_with_unsafe_fixes} hidden fixes")
                    };
                    format!(
                        "{n_diagnostic_with_fixes} fixable with the `--fix` option ({unsafe_label} can be enabled with the `--unsafe-fixes` option)."
                    )
                };
                println!("{msg}");
            } else if n_diagnostic_with_unsafe_fixes > 0 {
                let label = if n_diagnostic_with_unsafe_fixes == 1 {
                    "1 fix is".to_string()
                } else {
                    format!("{n_diagnostic_with_unsafe_fixes} fixes are")
                };
                println!("{label} available with the `--fix --unsafe-fixes` option.");
            }
        } else if errors.is_empty() {
            println!("All checks passed!");
        }

        Ok(())
    }
}

pub struct JsonEmitter;

impl Emitter for JsonEmitter {
    fn emit<W: Write>(
        &self,
        writer: &mut W,
        diagnostics: &Vec<&Diagnostic>,
        _errors: &Vec<(String, anyhow::Error)>,
    ) -> anyhow::Result<()> {
        serde_json::to_writer_pretty(writer, diagnostics)?;
        Ok(())
    }
}
