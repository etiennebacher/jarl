use air_fs::relativize_path;
use annotate_snippets::{Level, Renderer, Snippet};
use clap::ValueEnum;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;

use jarl_core::diagnostic::Diagnostic;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
pub enum OutputFormat {
    #[default]
    /// Print diagnostics with full context using annotated code snippets
    Full,
    /// Print diagnostics in a concise format, one per line
    Concise,
    /// Print diagnostics as GitHub format
    Github,
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
        diagnostics: &[&Diagnostic],
        errors: &[(String, anyhow::Error)],
    ) -> anyhow::Result<()>;
}

pub struct ConciseEmitter;

impl Emitter for ConciseEmitter {
    fn emit<W: Write>(
        &self,
        writer: &mut W,
        diagnostics: &[&Diagnostic],
        errors: &[(String, anyhow::Error)],
    ) -> anyhow::Result<()> {
        let mut total_diagnostics = 0;
        let mut n_diagnostic_with_fixes = 0usize;
        let mut n_diagnostic_with_unsafe_fixes = 0usize;

        // First, print all parsing errors
        if !errors.is_empty() {
            for (_path, err) in errors {
                let root_cause = err.chain().last().unwrap();
                if root_cause.is::<jarl_core::error::ParseError>() {
                    eprintln!("{}: {}", "Error".red().bold(), root_cause);
                } else {
                    eprintln!("{}: {}", "Error".red().bold(), err);
                }
            }
        }

        // Then, print the diagnostics.
        for diagnostic in diagnostics {
            let (row, col) = match diagnostic.location {
                Some(loc) => (loc.row(), loc.column() + 1), // Convert to 1-based for display
                None => {
                    unreachable!("Row/col locations must have been parsed successfully before.")
                }
            };
            let message = if let Some(suggestion) = &diagnostic.message.suggestion {
                format!("{} {}", diagnostic.message.body, suggestion)
            } else {
                diagnostic.message.body.clone()
            };
            writeln!(
                writer,
                "{} [{}:{}] {} {}",
                relativize_path(diagnostic.filename.clone()).white(),
                row,
                col,
                diagnostic.message.name.red(),
                message
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
                println!("\nFound {total_diagnostics} errors.");
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
        diagnostics: &[&Diagnostic],
        _errors: &[(String, anyhow::Error)],
    ) -> anyhow::Result<()> {
        serde_json::to_writer_pretty(writer, diagnostics)?;
        Ok(())
    }
}

pub struct GithubEmitter;

impl Emitter for GithubEmitter {
    fn emit<W: Write>(
        &self,
        writer: &mut W,
        diagnostics: &[&Diagnostic],
        _errors: &[(String, anyhow::Error)],
    ) -> anyhow::Result<()> {
        for diagnostic in diagnostics {
            let (row, col) = match diagnostic.location {
                Some(loc) => (loc.row(), loc.column() + 1), // Convert to 1-based for display
                None => {
                    unreachable!("Row/col locations must have been parsed successfully before.")
                }
            };

            write!(
                writer,
                "::warning file={file},line={row},col={col}::",
                file = diagnostic.filename.to_string_lossy()
            )?;

            let message = if let Some(suggestion) = &diagnostic.message.suggestion {
                format!("{} {}", diagnostic.message.body, suggestion)
            } else {
                diagnostic.message.body.clone()
            };
            writeln!(writer, "[{}] {}", diagnostic.message.name, message)?;
        }

        Ok(())
    }
}

pub struct FullEmitter;

impl Emitter for FullEmitter {
    fn emit<W: Write>(
        &self,
        writer: &mut W,
        diagnostics: &[&Diagnostic],
        errors: &[(String, anyhow::Error)],
    ) -> anyhow::Result<()> {
        // Use plain renderer when NO_COLOR is set or in snapshots
        let renderer = if std::env::var("NO_COLOR").is_ok() {
            Renderer::plain()
        } else {
            Renderer::styled()
        };
        let mut total_diagnostics = 0;
        let mut n_diagnostic_with_fixes = 0usize;
        let mut n_diagnostic_with_unsafe_fixes = 0usize;

        // First, print all parsing errors
        if !errors.is_empty() {
            for (_path, err) in errors {
                let root_cause = err.chain().last().unwrap();
                if root_cause.is::<jarl_core::error::ParseError>() {
                    eprintln!("{}: {}", "Error".red().bold(), root_cause);
                } else {
                    eprintln!("{}: {}", "Error".red().bold(), err);
                }
            }
            if !diagnostics.is_empty() {
                eprintln!(); // Add separator between errors and diagnostics
            }
        }

        // Group diagnostics by file for efficient file reading
        let mut diagnostics_by_file: std::collections::HashMap<&std::path::Path, Vec<&Diagnostic>> =
            std::collections::HashMap::new();

        for diagnostic in diagnostics {
            diagnostics_by_file
                .entry(diagnostic.filename.as_path())
                .or_default()
                .push(diagnostic);
        }

        // Process each file's diagnostics
        for diagnostic in diagnostics {
            let (_row, _col) = match diagnostic.location {
                Some(loc) => (loc.row(), loc.column()),
                None => {
                    unreachable!("Row/col locations must have been parsed successfully before.")
                }
            };

            // Read the source file
            let source = match fs::read_to_string(&diagnostic.filename) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!(
                        "Warning: Could not read source file {}: {}",
                        diagnostic.filename.display(),
                        e
                    );
                    continue;
                }
            };

            // Calculate the byte offset from TextRange
            let start_offset = diagnostic.range.start().into();
            let end_offset = diagnostic.range.end().into();

            // Create the snippet with annotate-snippets
            let file_path = relativize_path(diagnostic.filename.clone());

            // Build the message with snippet
            let snippet = Snippet::source(&source)
                .origin(&file_path)
                .fold(true)
                .annotation(
                    Level::Warning
                        .span(start_offset..end_offset)
                        .label(&diagnostic.message.body),
                );

            // Create the main message
            let mut message = Level::Warning
                .title(&diagnostic.message.name)
                .snippet(snippet);

            // Add suggestion as a footer message if present
            if let Some(suggestion_text) = &diagnostic.message.suggestion {
                message = message.footer(Level::Help.title(suggestion_text));
            }

            let rendered = renderer.render(message);
            writeln!(writer, "{rendered}\n")?;

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
                println!("Found {total_diagnostics} errors.");
            } else {
                println!("Found 1 error.");
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
