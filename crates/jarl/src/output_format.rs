use air_fs::relativize_path;
use annotate_snippets::Renderer;
use clap::ValueEnum;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufWriter, Write};

/// Creates a terminal hyperlink using OSC 8 escape sequences
/// Format: \x1b]8;;<URL>\x1b\\<TEXT>\x1b]8;;\x1b\\
fn make_hyperlink(text: &str) -> String {
    format!(
        "\x1b]8;;{}{}\x1b\\{}\x1b]8;;\x1b\\",
        "https://jarl.etiennebacher.com/rules/", text, text
    )
}

use jarl_core::diagnostic::{Diagnostic, render_diagnostic};

fn show_hint_statistics(total_diagnostics: i32) {
    let n_violations = std::env::var("JARL_N_VIOLATIONS_HINT_STAT")
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(15);
    if total_diagnostics > n_violations {
        println!(
            "\nMore than {n_violations} errors reported, use `--statistics` to get the count by rule."
        );
    }
}

#[derive(Debug, Serialize)]
struct JsonOutput<'a> {
    diagnostics: Vec<&'a Diagnostic>,
    errors: Vec<JsonError>,
}

#[derive(Debug, Serialize)]
struct JsonError {
    file: String,
    error: String,
}

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
        let mut writer = BufWriter::new(writer);
        let mut total_diagnostics = 0;
        let mut n_diagnostic_with_fixes = 0usize;
        let mut n_diagnostic_with_unsafe_fixes = 0usize;

        // First, print all parsing errors
        if !errors.is_empty() {
            writer.flush()?; // Flush before writing to stderr
            for (_path, err) in errors {
                let root_cause = err.chain().last().unwrap();
                if root_cause.is::<jarl_core::error::ParseError>() {
                    eprintln!("{}: {}", "Error".red().bold(), root_cause);
                } else {
                    eprintln!("{}: {}", "Error".red().bold(), err);
                }
            }
        }

        // Cache relativized paths to avoid repeated filesystem operations
        let mut path_cache = std::collections::HashMap::new();

        // Then, print the diagnostics.
        for diagnostic in diagnostics {
            let (row, col) = match diagnostic.location {
                Some(loc) => (loc.row(), loc.column() + 1), // Convert to 1-based for display
                None => {
                    unreachable!("Row/col locations must have been parsed successfully before.")
                }
            };

            // Get or compute relativized path
            let relative_path = path_cache
                .entry(&diagnostic.filename)
                .or_insert_with(|| relativize_path(diagnostic.filename.clone()));

            let message = if let Some(suggestion) = &diagnostic.message.suggestion {
                format!("{} {}", diagnostic.message.body, suggestion)
            } else {
                diagnostic.message.body.clone()
            };
            let use_colors = std::env::var("NO_COLOR").is_err();
            let rule_name = if use_colors {
                &make_hyperlink(&diagnostic.message.name)
            } else {
                &diagnostic.message.name
            };
            writeln!(
                writer,
                "{} [{}:{}] {} {}",
                relative_path.white(),
                row,
                col,
                rule_name.red(),
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

        writer.flush()?; // Ensure all diagnostics are written before summary

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

            show_hint_statistics(total_diagnostics);
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
        errors: &[(String, anyhow::Error)],
    ) -> anyhow::Result<()> {
        let mut writer = BufWriter::new(writer);

        // Convert errors to a serializable format
        let json_errors: Vec<JsonError> = errors
            .iter()
            .map(|(path, err)| JsonError { file: path.clone(), error: format!("{:#}", err) })
            .collect();

        let output = JsonOutput {
            diagnostics: diagnostics.to_vec(),
            errors: json_errors,
        };

        serde_json::to_writer_pretty(&mut writer, &output)?;
        writer.flush()?;
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
        let mut writer = BufWriter::new(writer);
        for diagnostic in diagnostics {
            let (row, col) = match diagnostic.location {
                Some(loc) => (loc.row(), loc.column() + 1), // Convert to 1-based for display
                None => {
                    unreachable!("Row/col locations must have been parsed successfully before.")
                }
            };

            // We want a message like this:
            // ::warning title=Jarl (any_is_na),file=demos/foo.R,line=4,col=5::demos/foo.R:4:5: any_is_na `any(is.na(...))` etc.
            //
            // The location appears twice:
            // - one between the "::" markers: this is for the annotation to
            //   appear when we browse changed files in Github PR;
            // - one after the "::" marker: this is so that the workflow shows
            //   the location of diagnostics when we inspect the workflow itself,
            //   without the Github annotations.
            write!(
                writer,
                "::warning title=Jarl ({}),file={file},line={row},col={col}::{file}:{row}:{col} ",
                diagnostic.message.name,
                file = diagnostic.filename.to_string_lossy()
            )?;

            let message = if let Some(suggestion) = &diagnostic.message.suggestion {
                format!("{} {}", diagnostic.message.body, suggestion)
            } else {
                diagnostic.message.body.clone()
            };
            writeln!(writer, "[{}] {}", diagnostic.message.name, message)?;
        }

        writer.flush()?;
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
        let mut writer = BufWriter::new(writer);
        // Use plain renderer when NO_COLOR is set or in snapshots
        let use_colors = std::env::var("NO_COLOR").is_err();
        let renderer = if use_colors {
            Renderer::styled()
        } else {
            Renderer::plain()
        };
        let mut total_diagnostics = 0;
        let mut n_diagnostic_with_fixes = 0usize;
        let mut n_diagnostic_with_unsafe_fixes = 0usize;

        // First, print all parsing errors
        if !errors.is_empty() {
            writer.flush()?; // Flush before writing to stderr
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

        // Cache file contents and relativized paths
        let mut file_cache: std::collections::HashMap<&std::path::Path, String> =
            std::collections::HashMap::new();
        let mut path_cache = std::collections::HashMap::new();

        // Pre-load all files into cache
        for diagnostic in diagnostics {
            if !file_cache.contains_key(diagnostic.filename.as_path()) {
                match fs::read_to_string(&diagnostic.filename) {
                    Ok(content) => {
                        file_cache.insert(diagnostic.filename.as_path(), content);
                    }
                    Err(err) => {
                        writer.flush()?; // Flush before writing to stderr
                        eprintln!(
                            "Warning: Could not read source file {}: {}",
                            diagnostic.filename.display(),
                            err
                        );
                    }
                }
            }
        }

        // Process each file's diagnostics
        for diagnostic in diagnostics {
            let (_row, _col) = match diagnostic.location {
                Some(loc) => (loc.row(), loc.column()),
                None => {
                    unreachable!("Row/col locations must have been parsed successfully before.")
                }
            };

            // Get the source file from cache
            let Some(source) = file_cache.get(diagnostic.filename.as_path()) else {
                continue; // Skip if file couldn't be read
            };

            // Get or compute relativized path
            let file_path = path_cache
                .entry(&diagnostic.filename)
                .or_insert_with(|| relativize_path(diagnostic.filename.clone()));

            // Create the main message with clickable rule name
            let title = if use_colors {
                make_hyperlink(&diagnostic.message.name)
            } else {
                diagnostic.message.name.clone()
            };

            let rendered = render_diagnostic(source, file_path, &title, diagnostic, &renderer);
            writeln!(writer, "{rendered}\n")?;

            if diagnostic.has_safe_fix() {
                n_diagnostic_with_fixes += 1;
            }
            if diagnostic.has_unsafe_fix() {
                n_diagnostic_with_unsafe_fixes += 1;
            }
            total_diagnostics += 1;
        }

        writer.flush()?; // Ensure all diagnostics are written before summary

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

            show_hint_statistics(total_diagnostics);
        } else if errors.is_empty() {
            println!("All checks passed!");
        }

        Ok(())
    }
}
