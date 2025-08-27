use air_workspace::discovery::DiscoveredSettings;
use air_workspace::discovery::discover_r_file_paths;
use air_workspace::discovery::discover_settings;
use air_workspace::resolve::PathResolver;
use air_workspace::settings::Settings;

use colored::Colorize;
use flir::args::CliArgs;
use flir::check::check;
use flir::config::build_config;
use flir::emitter::*;
use flir::error::ParseError;
use flir::message::Diagnostic;

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

    let mut total_diagnostics = 0;
    let mut n_diagnostic_with_fixes = 0usize;
    let mut n_diagnostic_with_unsafe_fixes = 0usize;

    // Flatten all diagnostics into a single vector and sort globally
    let mut all_diagnostics_flat: Vec<&Diagnostic> = all_diagnostics
        .iter()
        .flat_map(|(_path, diagnostics)| diagnostics.iter())
        .collect();

    all_diagnostics_flat.sort();

    let mut stdout = std::io::stdout();
    match args.output_format {
        OutputFormat::Json => {
            JsonEmitter.emit(&mut stdout, &all_diagnostics_flat)?;
            return Ok(ExitCode::from(1));
        }
        _ => {}
    }

    // First, print all parsing errors
    if !all_errors.is_empty() {
        for (_path, err) in &all_errors {
            let root_cause = err.chain().last().unwrap();
            if root_cause.is::<ParseError>() {
                eprintln!("{}: {}", "Error".red().bold(), root_cause);
            } else {
                eprintln!("{}: {}", "Error".red().bold(), err);
            }
        }
    }

    // Then, print all diagnostics
    for message in &all_diagnostics_flat {
        if message.has_safe_fix() {
            n_diagnostic_with_fixes += 1;
        }
        if message.has_unsafe_fix() {
            n_diagnostic_with_unsafe_fixes += 1;
        }
        println!("{message}");
        total_diagnostics += 1;
    }

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
    } else if all_errors.is_empty() {
        println!("All checks passed!");
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
