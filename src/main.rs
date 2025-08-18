use air_workspace::discovery::DiscoveredSettings;
use air_workspace::discovery::discover_r_file_paths;
use air_workspace::discovery::discover_settings;
use air_workspace::resolve::PathResolver;
use air_workspace::settings::Settings;

use colored::Colorize;
use flir::args::CliArgs;
use flir::check::check;
use flir::config::build_config;

use anyhow::Result;
use clap::Parser;
use std::time::Instant;

fn main() -> Result<()> {
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
        return Ok(());
    }

    // use std::path::Path;
    // let paths = vec![Path::new("demos/foo.R").to_path_buf()];

    let config = build_config(&args, paths)?;

    let mut diagnostics = check(config)?;

    if !args.fix && !diagnostics.is_empty() {
        let mut n_diagnostic_with_fixes = 0usize;
        let mut n_diagnostic_with_unsafe_fixes = 0usize;
        diagnostics.sort();
        for message in &diagnostics {
            if message.has_safe_fix() {
                n_diagnostic_with_fixes += 1;
            }
            if message.has_unsafe_fix() {
                n_diagnostic_with_unsafe_fixes += 1;
            }
            println!("{message}");
        }

        if diagnostics.len() > 1 {
            println!("\nFound {} errors.", diagnostics.len())
        } else {
            println!("\nFound 1 error.")
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
                format!("{} fixes are", n_diagnostic_with_unsafe_fixes)
            };
            println!("{label} available with the `--fix --unsafe-fixes` option.");
        }
    } else {
        println!("All checks passed!")
    }

    if let Some(start) = start {
        let duration = start.elapsed();
        println!("\nChecked files in: {duration:?}");
    }

    Ok(())
}
