use air_workspace::resolve::PathResolver;
use jarl_core::discovery::{discover_r_file_paths, discover_settings};
use jarl_core::rule_set::Rule;
use jarl_core::{
    config::ArgsConfig,
    config::build_config,
    diagnostic::Diagnostic,
    settings::Settings,
    suppression_edit::{create_suppression_edit, format_suppression_comments},
};

use anyhow::Result;
use colored::Colorize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
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

    // Emit deprecation warnings for old assignment syntax
    if check_config.assignment.is_some() {
        eprintln!(
            "{}: `--assignment` is deprecated. Use `[lint.assignment]` in jarl.toml instead.",
            "Warning".yellow().bold()
        );
    }

    // Check if the deprecated `assignment = "..."` top-level string form was used in TOML
    for item in resolver.items() {
        if item.value().linter.deprecated_assignment_syntax {
            eprintln!(
                "{}: `assignment = \"...\"` in `[lint]` is deprecated. \
                 Use `[lint.assignment]` with `operator = \"...\"` instead.",
                "Warning".yellow().bold()
            );
        }
    }

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
        // Emit deprecation warnings for explicitly-used deprecated rules.
        // Collect rule names from CLI args and TOML settings.
        let mut explicit_rule_names: BTreeSet<String> = BTreeSet::new();

        // CLI args (comma-separated)
        for arg_str in [&args.select, &args.extend_select, &args.ignore] {
            for name in arg_str.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                explicit_rule_names.insert(name.to_string());
            }
        }

        // TOML settings from all discovered configs
        for item in resolver.items() {
            let linter = &item.value().linter;
            for names in [&linter.select, &linter.extend_select, &linter.ignore]
                .into_iter()
                .flatten()
            {
                for name in names {
                    explicit_rule_names.insert(name.clone());
                }
            }
        }

        for name in &explicit_rule_names {
            if let Some(rule) = Rule::from_name(name)
                && let Some(dep) = rule.deprecation()
            {
                eprintln!(
                    "{}: Rule `{}` is deprecated since v{}. Use `{}` instead.",
                    "Warning".yellow().bold(),
                    name,
                    dep.version,
                    dep.replacement,
                );
            }
        }

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
        // Store (offset, indent, needs_leading_newline, rule_name) to merge rules at same offset
        let mut raw_edits: Vec<(usize, String, bool, String)> = Vec::new();
        for diagnostic in &diagnostics {
            let start: usize = diagnostic.range.start().into();
            let end: usize = diagnostic.range.end().into();
            let rule_name = &diagnostic.message.name;

            if let Some(edit) = create_suppression_edit(&content, start, end, rule_name, reason) {
                raw_edits.push((
                    edit.insert_point.offset,
                    edit.insert_point.indent,
                    edit.insert_point.needs_leading_newline,
                    rule_name.clone(),
                ));
            }
        }

        if raw_edits.is_empty() {
            continue;
        }

        // Sort by offset ascending to group edits at the same offset
        raw_edits.sort_by(|a, b| a.0.cmp(&b.0));

        // Merge edits at the same offset: collect all rule names for each offset
        let mut merged_edits: Vec<(usize, String, bool, Vec<String>)> = Vec::new();
        for (offset, indent, needs_leading_newline, rule_name) in raw_edits {
            if let Some(last) = merged_edits.last_mut()
                && last.0 == offset
            {
                // Same offset - add rule if not already present
                if !last.3.contains(&rule_name) {
                    last.3.push(rule_name);
                }
                continue;
            }
            // New offset
            merged_edits.push((offset, indent, needs_leading_newline, vec![rule_name]));
        }

        // Sort by offset in descending order so we can apply edits without shifting positions
        merged_edits.sort_by(|a, b| b.0.cmp(&a.0));

        // Apply edits to the content
        let mut modified_content = content.clone();
        for (offset, indent, needs_leading_newline, rule_names) in &merged_edits {
            let rule_refs: Vec<&str> = rule_names.iter().map(|s| s.as_str()).collect();
            let comment_text =
                format_suppression_comments(&rule_refs, reason, indent, *needs_leading_newline);
            modified_content.insert_str(*offset, &comment_text);
        }

        // Count total suppression comments (one per rule)
        let num_suppressions: usize = merged_edits
            .iter()
            .map(|(_, _, _, rules)| rules.len())
            .sum();

        // Write the modified content back
        match std::fs::write(&path, &modified_content) {
            Ok(()) => {
                total_suppressions += num_suppressions;
                files_modified += 1;
                println!(
                    "{}: Added {} suppression comment(s) to {}",
                    "Modified".green().bold(),
                    num_suppressions,
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
