use air_workspace::resolve::PathResolver;
use jarl_core::discovery::{discover_r_file_paths, discover_settings};
use jarl_core::{
    config::ArgsConfig, config::build_config, diagnostic::Diagnostic, settings::Settings,
};

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::time::Instant;

use crate::args::CheckCommand;
use crate::output_format::{self, GithubEmitter};
use crate::status::ExitStatus;

use output_format::{ConciseEmitter, Emitter, FullEmitter, JsonEmitter, OutputFormat};

pub fn check() -> Result<ExitStatus> {
    let args = CheckCommand::parse();

    let start = if args.with_timing {
        Some(Instant::now())
    } else {
        None
    };

    let mut resolver = PathResolver::new(Settings::default());

    // Load discovered settings. If the user passed `--no-default-exclude`,
    // override each discovered settings' `default_exclude` to `false` so the
    // default patterns from `DEFAULT_EXCLUDE_PATTERNS` are not applied during
    // discovery.
    for mut ds in discover_settings(&args.files)? {
        if args.no_default_exclude {
            ds.settings.linter.default_exclude = Some(false);
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

    // use std::path::Path;
    // let paths = vec![Path::new("demos/foo.R").to_path_buf()];

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

    // Flatten all diagnostics into a single vector and sort globally
    let mut all_diagnostics_flat: Vec<&Diagnostic> = all_diagnostics
        .iter()
        .flat_map(|(_path, diagnostics)| diagnostics.iter())
        .collect();

    all_diagnostics_flat.sort();

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

    if let Some(start) = start {
        let duration = start.elapsed();
        println!("\nChecked files in: {duration:?}");
    }

    if !all_errors.is_empty() {
        return Ok(ExitStatus::Error);
    }

    if all_diagnostics.is_empty() {
        return Ok(ExitStatus::Success);
    }

    Ok(ExitStatus::Failure)
}
