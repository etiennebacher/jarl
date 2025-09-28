use air_workspace::resolve::PathResolver;
use flir_core::discovery::{DiscoveredSettings, discover_r_file_paths, discover_settings};
use flir_core::{CheckConfig, Diagnostic, Settings, build_config, check};

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::process::ExitCode;
use std::time::Instant;

mod args;
mod output_format;

use args::CliArgs;
use output_format::{ConciseEmitter, Emitter, JsonEmitter, OutputFormat};

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode> {
    let args = CliArgs::parse();

    let start = if args.with_timing {
        Some(Instant::now())
    } else {
        None
    };

    let mut resolver = PathResolver::new(Settings::default());
    for DiscoveredSettings { directory, settings } in discover_settings(&args.files)? {
        resolver.add(&directory, settings);
    }

    let paths = discover_r_file_paths(&args.files, &resolver, true)
        .into_iter()
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    if paths.is_empty() {
        println!(
            "{}: {}",
            "Warning".yellow().bold(),
            "No R files found under the given path(s).".white().bold()
        );
        return Ok(ExitCode::from(0));
    }

    // use std::path::Path;
    // let paths = vec![Path::new("demos/foo.R").to_path_buf()];

    let check_config = CheckConfig {
        files: args.files.iter().map(|s| s.into()).collect(),
        fix: args.fix,
        unsafe_fixes: args.unsafe_fixes,
        fix_only: args.fix_only,
        select_rules: args.select_rules.clone(),
        ignore_rules: args.ignore_rules.clone(),
        min_r_version: args.min_r_version.clone(),
    };

    let config = build_config(&check_config, &resolver, paths)?;

    let file_results = check(config);

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
        OutputFormat::Json => {
            JsonEmitter.emit(&mut stdout, &all_diagnostics_flat, &all_errors)?;
        }
        OutputFormat::Concise => {
            ConciseEmitter.emit(&mut stdout, &all_diagnostics_flat, &all_errors)?;
        }
    }

    if !all_errors.is_empty() {
        return Ok(ExitCode::from(1));
    }

    if all_diagnostics.is_empty() {
        return Ok(ExitCode::from(0));
    }

    if let Some(start) = start {
        let duration = start.elapsed();
        println!("\nChecked files in: {duration:?}");
    }

    Ok(ExitCode::from(1))
}
