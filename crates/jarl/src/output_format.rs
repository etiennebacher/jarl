use air_fs::relativize_path;
use annotate_snippets::Renderer;
use clap::ValueEnum;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
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

/// Prints a section header like `── Summary ──────────────────────────────────`
/// padded to 57 characters total.
pub fn print_section_header(title: &str) {
    const TOTAL_WIDTH: usize = 57;
    // "── {title} ──" takes up 5 + title.len() chars (2 for ── , 1 space, 1 space, 2 for ──)
    let prefix = format!("── {title} ──");
    let pad = TOTAL_WIDTH.saturating_sub(prefix.len());
    let padding: String = "─".repeat(pad);
    println!("{prefix}{padding}");
}

/// Prints the summary section with error counts and fix info.
/// Only call for human-readable formats (Full, Concise).
pub fn print_summary(diagnostics: &[&Diagnostic], has_errors: bool) {
    let total: i32 = diagnostics.len() as i32;
    let n_safe_fixes = diagnostics.iter().filter(|d| d.has_safe_fix()).count();
    let n_unsafe_fixes = diagnostics.iter().filter(|d| d.has_unsafe_fix()).count();

    if total > 0 {
        println!();
        print_section_header("Summary");

        if total > 1 {
            println!("Found {total} errors.");
        } else {
            println!("Found 1 error.");
        }

        if n_safe_fixes > 0 {
            let msg = if n_unsafe_fixes == 0 {
                format!("{n_safe_fixes} fixable with the `--fix` option.")
            } else {
                let unsafe_label = if n_unsafe_fixes == 1 {
                    "1 hidden fix".to_string()
                } else {
                    format!("{n_unsafe_fixes} hidden fixes")
                };
                format!(
                    "{n_safe_fixes} fixable with the `--fix` option ({unsafe_label} can be enabled with the `--unsafe-fixes` option)."
                )
            };
            println!("{msg}");
        } else if n_unsafe_fixes > 0 {
            let label = if n_unsafe_fixes == 1 {
                "1 fix is".to_string()
            } else {
                format!("{n_unsafe_fixes} fixes are")
            };
            println!("{label} available with the `--fix --unsafe-fixes` option.");
        }

        let n_violations = std::env::var("JARL_N_VIOLATIONS_HINT_STAT")
            .ok()
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(15);
        if total > n_violations {
            println!(
                "More than {n_violations} errors reported, use `--statistics` to get the count by rule."
            );
        }
    } else if !has_errors {
        print_section_header("Summary");
        println!("All checks passed!");
    }
}

/// Prints warnings under a `── Warnings ──` section header.
pub fn print_warnings(warnings: &[String]) {
    if warnings.is_empty() {
        return;
    }
    println!();
    print_section_header("Warnings");
    for warning in warnings {
        println!("{warning}");
    }
}

/// Prints notes under a `── Notes ──` section header.
pub fn print_notes(notes: &[String]) {
    if notes.is_empty() {
        return;
    }
    println!();
    print_section_header("Notes");
    for note in notes {
        println!("{note}");
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
    /// Print diagnostics as SARIF 2.1.0 JSON
    Sarif,
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
        }

        writer.flush()?;
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

/// An emitter producing SARIF 2.1.0-compliant JSON output.
///
/// Static Analysis Results Interchange Format (SARIF) is a standard format for
/// static analysis results, consumed by tools such as GitHub Code Scanning. See
/// [SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html).
pub struct SarifEmitter;

const SARIF_HELP_URI_BASE: &str = "https://jarl.etiennebacher.com/rules/";

#[derive(Debug, Serialize)]
struct SarifOutput<'a> {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: [SarifRun<'a>; 1],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRun<'a> {
    tool: SarifTool<'a>,
    /// Columns are reported in UTF-16 code units, matching the SARIF default
    /// and the convention used by other R linters (e.g. lintr).
    column_kind: &'static str,
    original_uri_base_ids: OriginalUriBaseIds,
    results: Vec<SarifResult<'a>>,
}

/// Base URIs that result locations are resolved against. Jarl uses a single
/// `ROOTPATH` pointing at the current working directory, so each result's `uri`
/// is stored relative to it.
#[derive(Debug, Serialize)]
struct OriginalUriBaseIds {
    #[serde(rename = "ROOTPATH")]
    root_path: SarifUriBase,
}

#[derive(Debug, Serialize)]
struct SarifUriBase {
    uri: String,
}

#[derive(Debug, Serialize)]
struct SarifTool<'a> {
    driver: SarifDriver<'a>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifDriver<'a> {
    name: &'static str,
    information_uri: &'static str,
    version: &'static str,
    rules: Vec<SarifRule<'a>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRule<'a> {
    id: &'a str,
    short_description: SarifMessage<'a>,
    help: SarifMessage<'a>,
    help_uri: String,
    default_configuration: SarifDefaultConfiguration,
}

#[derive(Debug, Serialize)]
struct SarifDefaultConfiguration {
    level: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifResult<'a> {
    rule_id: &'a str,
    rule_index: usize,
    level: &'static str,
    message: SarifMessage<'a>,
    locations: [SarifLocation; 1],
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fixes: Vec<SarifFix>,
}

#[derive(Debug, Serialize)]
struct SarifMessage<'a> {
    text: Cow<'a, str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifLocation {
    physical_location: SarifPhysicalLocation,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifPhysicalLocation {
    artifact_location: SarifArtifactLocation,
    region: SarifRegion,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifArtifactLocation {
    uri: String,
    uri_base_id: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRegion {
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifFix {
    description: SarifMessage<'static>,
    artifact_changes: [SarifArtifactChange; 1],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifArtifactChange {
    artifact_location: SarifArtifactLocation,
    replacements: [SarifReplacement; 1],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifReplacement {
    deleted_region: SarifRegion,
    #[serde(skip_serializing_if = "Option::is_none")]
    inserted_content: Option<SarifMessage<'static>>,
}

/// Compute the 1-indexed line and column of a byte `offset` within `content`.
///
/// The column is measured in UTF-16 code units (the SARIF default declared via
/// `columnKind`), so it is correct even when the line contains non-ASCII
/// characters. `offset` must fall on a UTF-8 char boundary, which holds for the
/// byte offsets produced by the parser.
fn offset_to_line_column(content: &str, offset: usize) -> (usize, usize) {
    let before = &content[..offset];
    let line = before.bytes().filter(|&b| b == b'\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |p| p + 1);
    let column = content[line_start..offset]
        .chars()
        .map(char::len_utf16)
        .sum::<usize>()
        + 1;
    (line, column)
}

/// Convert a byte range into a 1-indexed SARIF region (UTF-16 columns).
fn range_to_region(content: &str, start: usize, end: usize) -> SarifRegion {
    let (start_line, start_column) = offset_to_line_column(content, start);
    let (end_line, end_column) = offset_to_line_column(content, end);
    SarifRegion { start_line, start_column, end_line, end_column }
}

impl Emitter for SarifEmitter {
    fn emit<W: Write>(
        &self,
        writer: &mut W,
        diagnostics: &[&Diagnostic],
        _errors: &[(String, anyhow::Error)],
    ) -> anyhow::Result<()> {
        let mut writer = BufWriter::new(writer);

        // Cache each file's contents so ranges can be converted to line/column
        // regions without re-reading the source.
        let mut content_cache: std::collections::HashMap<std::path::PathBuf, String> =
            std::collections::HashMap::new();

        // Collect unique rules (sorted by name) using the first diagnostic body
        // we see as the rule's short description, since Jarl has no static
        // per-rule description text.
        let mut rule_bodies: std::collections::BTreeMap<&str, &str> =
            std::collections::BTreeMap::new();
        for diagnostic in diagnostics {
            rule_bodies
                .entry(&diagnostic.message.name)
                .or_insert(&diagnostic.message.body);
        }
        let rules: Vec<SarifRule> = rule_bodies
            .into_iter()
            .map(|(name, body)| SarifRule {
                id: name,
                short_description: SarifMessage { text: Cow::Borrowed(body) },
                help: SarifMessage { text: Cow::Borrowed(body) },
                help_uri: format!("{SARIF_HELP_URI_BASE}{name}"),
                default_configuration: SarifDefaultConfiguration { level: "warning" },
            })
            .collect();

        // Map each rule name to its index in `rules` so results can reference it
        // via `ruleIndex`.
        let rule_indices: std::collections::HashMap<&str, usize> = rules
            .iter()
            .enumerate()
            .map(|(index, rule)| (rule.id, index))
            .collect();

        let mut results = Vec::with_capacity(diagnostics.len());
        for diagnostic in diagnostics {
            let content = match content_cache.entry(diagnostic.filename.clone()) {
                std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::hash_map::Entry::Vacant(entry) => {
                    let Ok(content) = fs::read_to_string(&diagnostic.filename) else {
                        continue;
                    };
                    entry.insert(content)
                }
            };

            let uri = relativize_path(diagnostic.filename.clone())
                .replace('\\', "/");

            let region = range_to_region(
                content,
                diagnostic.range.start().into(),
                diagnostic.range.end().into(),
            );

            let message = if let Some(suggestion) = &diagnostic.message.suggestion {
                format!("{} {}", diagnostic.message.body, suggestion)
            } else {
                diagnostic.message.body.clone()
            }
            .replace('\\', "/");

            // A fix is only emitted when it edits the source (not skipped, and
            // it either inserts content or deletes a non-empty range).
            let fix = &diagnostic.fix;
            let fixes = if !fix.to_skip && (fix.start != fix.end || !fix.content.is_empty()) {
                let deleted_region = range_to_region(content, fix.start, fix.end);
                let inserted_content = (!fix.content.is_empty())
                    .then(|| SarifMessage { text: Cow::Owned(fix.content.clone()) });
                vec![SarifFix {
                    description: SarifMessage { text: Cow::Owned(message.clone()) },
                    artifact_changes: [SarifArtifactChange {
                        artifact_location: SarifArtifactLocation {
                            uri: uri.clone(),
                            uri_base_id: "ROOTPATH",
                        },
                        replacements: [SarifReplacement { deleted_region, inserted_content }],
                    }],
                }]
            } else {
                Vec::new()
            };

            results.push(SarifResult {
                rule_id: &diagnostic.message.name,
                rule_index: rule_indices[diagnostic.message.name.as_str()],
                level: "warning",
                message: SarifMessage { text: Cow::Owned(message) },
                locations: [SarifLocation {
                    physical_location: SarifPhysicalLocation {
                        artifact_location: SarifArtifactLocation { uri, uri_base_id: "ROOTPATH" },
                        region,
                    },
                }],
                fixes,
            });
        }

        // Base URI that result paths are resolved against. Paths are stored
        // relative to the current working directory.
        let root_uri = std::env::current_dir()
            .map(|dir| format!("file://{}/", dir.display().to_string().replace('\\', "/")))
            .unwrap_or_else(|_| "file://./".to_string());

        let output = SarifOutput {
            schema: "https://json.schemastore.org/sarif-2.1.0.json",
            version: "2.1.0",
            runs: [SarifRun {
                tool: SarifTool {
                    driver: SarifDriver {
                        name: "jarl",
                        information_uri: "https://github.com/etiennebacher/jarl",
                        version: env!("CARGO_PKG_VERSION"),
                        rules,
                    },
                },
                column_kind: "utf16CodeUnits",
                original_uri_base_ids: OriginalUriBaseIds {
                    root_path: SarifUriBase { uri: root_uri },
                },
                results,
            }],
        };

        serde_json::to_writer_pretty(&mut writer, &output)?;
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
        }

        writer.flush()?;
        Ok(())
    }
}
