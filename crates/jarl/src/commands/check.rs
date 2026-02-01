use air_workspace::resolve::PathResolver;
use jarl_core::discovery::{discover_r_file_paths, discover_settings};
use jarl_core::{
    config::ArgsConfig, config::build_config, diagnostic::Diagnostic, settings::Settings,
    suppression_edit::create_suppression_edit,
};

use anyhow::Result;
use colored::Colorize;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use std::time::Instant;

use crate::args::CheckCommand;
use crate::output_format::{self, GithubEmitter};
use crate::statistics::print_statistics;
use crate::status::ExitStatus;

use output_format::{ConciseEmitter, Emitter, FullEmitter, JsonEmitter, OutputFormat};

pub fn check(args: CheckCommand) -> Result<ExitStatus> {
    let start = if args.with_timing {
        Some(Instant::now())
    } else {
        None
    };

    let mut resolver = PathResolver::new(Settings::default());

    // Track if we're using a config from a parent directory
    let mut parent_config_path: Option<PathBuf> = None;
    let cwd = env::current_dir().ok();

    // Load discovered settings. If the user passed `--no-default-exclude`,
    // override each discovered settings' `default_exclude` to `false` so the
    // default patterns from `DEFAULT_EXCLUDE_PATTERNS` are not applied during
    // discovery.
    for mut ds in discover_settings(&args.files)? {
        if args.no_default_exclude {
            ds.settings.linter.default_exclude = Some(false);
        }

        // Check if config is from a parent directory (not CWD)
        if let (Some(config_path), Some(current_dir)) = (&ds.config_path, &cwd)
            && let Some(config_dir) = config_path.parent()
            && config_dir != current_dir
        {
            parent_config_path = Some(config_path.clone());
        }

        resolver.add(&ds.directory, ds.settings);
    }

    let paths = discover_r_file_paths(&args.files, &resolver, true, args.no_default_exclude)
        .into_iter()
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    if paths.is_empty() {
        println!(
            "{}: {}",
            "Warning".yellow().bold(),
            "No R files found under the given path(s).".white().bold()
        );
        return Ok(ExitStatus::Success);
    }

    let check_config = ArgsConfig {
        files: args.files.iter().map(|s| s.into()).collect(),
        fix: args.fix,
        unsafe_fixes: args.unsafe_fixes,
        fix_only: args.fix_only,
        select: args.select.clone(),
        extend_select: args.extend_select.clone(),
        ignore: args.ignore.clone(),
        min_r_version: args.min_r_version.clone(),
        allow_dirty: args.allow_dirty,
        allow_no_vcs: args.allow_no_vcs,
        assignment: args.assignment,
    };

    let config = build_config(&check_config, &resolver, paths)?;

    let file_results = jarl_core::check::check(config);

    let mut all_errors = Vec::new();
    let mut all_diagnostics = Vec::new();

    for (path, result) in file_results {
        match result {
            Ok(diagnostics) => {
                if !diagnostics.is_empty() {
                    all_diagnostics.push((path, diagnostics));
                }
            }
            Err(e) => {
                all_errors.push((path, e));
            }
        }
    }

    // Handle --add-jarl-ignore: insert suppression comments for all diagnostics
    if let Some(reason) = &args.add_jarl_ignore {
        return add_jarl_ignore_comments(&all_diagnostics, reason, parent_config_path);
    }

    // Flatten all diagnostics into a single vector and sort globally
    let mut all_diagnostics_flat: Vec<&Diagnostic> = all_diagnostics
        .iter()
        .flat_map(|(_path, diagnostics)| diagnostics.iter())
        .collect();

    all_diagnostics_flat.sort();

    if args.statistics {
        return print_statistics(&all_diagnostics_flat, parent_config_path);
    }

    let mut stdout = std::io::stdout();

    match args.output_format {
        OutputFormat::Concise => {
            ConciseEmitter.emit(&mut stdout, &all_diagnostics_flat, &all_errors)?;
        }
        OutputFormat::Json => {
            JsonEmitter.emit(&mut stdout, &all_diagnostics_flat, &all_errors)?;
        }
        OutputFormat::Github => {
            GithubEmitter.emit(&mut stdout, &all_diagnostics_flat, &all_errors)?;
        }
        OutputFormat::Full => {
            FullEmitter.emit(&mut stdout, &all_diagnostics_flat, &all_errors)?;
        }
    }

    // For human-readable formats, print timing and config info
    // Skip for JSON/GitHub to avoid corrupting structured output
    let is_structured_format = matches!(
        args.output_format,
        OutputFormat::Json | OutputFormat::Github
    );

    if !is_structured_format {
        // Inform the user if the config file used comes from a parent directory.
        if let Some(config_path) = parent_config_path {
            println!("\nUsed '{}'", config_path.display());
        }

        if let Some(start) = start {
            let duration = start.elapsed();
            println!("\nChecked files in: {duration:?}");
        }
    }

    if !all_errors.is_empty() {
        return Ok(ExitStatus::Error);
    }

    if all_diagnostics.is_empty() {
        return Ok(ExitStatus::Success);
    }

    Ok(ExitStatus::Failure)
}

/// Insert `# jarl-ignore` comments for all diagnostics in the given files.
fn add_jarl_ignore_comments(
    all_diagnostics: &[(String, Vec<Diagnostic>)],
    reason: &str,
    parent_config_path: Option<PathBuf>,
) -> Result<ExitStatus> {
    // Newlines would break comment format
    if reason.contains(['\n', '\r']) {
        return Err(anyhow::anyhow!(
            "--add-jarl-ignore=<reason> cannot contain newline characters."
        ));
    }

    if all_diagnostics.is_empty() {
        println!(
            "{}: {}",
            "Info".cyan().bold(),
            "No violations found, no suppression comments added.".white()
        );
        return Ok(ExitStatus::Success);
    }

    let mut total_suppressions = 0;
    let mut files_modified = 0;

    // Group diagnostics by file path (use BTreeMap for deterministic order)
    let mut by_file: BTreeMap<&str, Vec<&Diagnostic>> = BTreeMap::new();
    for (path, diagnostics) in all_diagnostics {
        by_file.entry(path).or_default().extend(diagnostics.iter());
    }

    for (path, diagnostics) in by_file {
        let path = PathBuf::from(path);
        // Read the file content
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "{}: Could not read {}: {}",
                    "Error".red().bold(),
                    path.display(),
                    e
                );
                continue;
            }
        };

        // Compute suppression edits for each diagnostic
        let mut edits: Vec<(usize, String)> = Vec::new();
        for diagnostic in &diagnostics {
            let start: usize = diagnostic.range.start().into();
            let end: usize = diagnostic.range.end().into();
            let rule_name = &diagnostic.message.name;

            if let Some(edit) = create_suppression_edit(&content, start, end, rule_name, reason) {
                edits.push((edit.insert_point.offset, edit.comment_text));
            }
        }

        if edits.is_empty() {
            continue;
        }

        // Sort by offset in descending order so we can apply edits without shifting positions
        edits.sort_by(|a, b| b.0.cmp(&a.0));

        // Deduplicate edits at the same offset (multiple diagnostics might want the same comment)
        edits.dedup_by(|a, b| a.0 == b.0);

        // Apply edits to the content
        let mut modified_content = content.clone();
        for (offset, comment_text) in &edits {
            modified_content.insert_str(*offset, comment_text);
        }

        // Write the modified content back
        match std::fs::write(&path, &modified_content) {
            Ok(()) => {
                total_suppressions += edits.len();
                files_modified += 1;
                println!(
                    "{}: Added {} suppression comment(s) to {}",
                    "Modified".green().bold(),
                    edits.len(),
                    path.display()
                );
            }
            Err(e) => {
                eprintln!(
                    "{}: Could not write {}: {}",
                    "Error".red().bold(),
                    path.display(),
                    e
                );
            }
        }
    }

    // Summary
    if total_suppressions > 0 {
        println!(
            "\n{}: Added {} suppression comment(s) across {} file(s).",
            "Summary".cyan().bold(),
            total_suppressions,
            files_modified
        );
    }

    // Inform about parent config if applicable
    if let Some(config_path) = parent_config_path {
        println!("\nUsed '{}'", config_path.display());
    }

    Ok(ExitStatus::Success)
}
