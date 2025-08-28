use air_workspace::discovery::DiscoveredSettings;
use air_workspace::discovery::discover_r_file_paths;
use air_workspace::discovery::discover_settings;
use air_workspace::resolve::PathResolver;
use air_workspace::settings::Settings;

use colored::Colorize;
use flir::args::CliArgs;
use flir::check::check;
use flir::config::build_config;
use flir::message::Diagnostic;
use flir::output_format::*;

use anyhow::Result;
use clap::Parser;
use flir::output_format::OutputFormat;
use std::process::ExitCode;
use std::time::Instant;

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
    for DiscoveredSettings { directory, settings } in discover_settings(&[args.dir.clone()])? {
        resolver.add(&directory, settings);
    }

    let paths = discover_r_file_paths(&[args.dir.clone()], &resolver, true)
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

    let config = build_config(&args, paths)?;

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
