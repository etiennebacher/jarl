use air_r_parser::RParserOptions;
use air_workspace::discovery::discover_r_file_paths;
use air_workspace::discovery::discover_settings;
use air_workspace::discovery::DiscoveredSettings;
use air_workspace::resolve::PathResolver;
use air_workspace::settings::Settings;

use flir::check::check;
use flir::config::build_config;

use anyhow::Result;
use clap::{arg, Parser};
use std::time::Instant;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(
    author,
    name = "flir",
    about = "flir: Find and Fix Lints in R Code",
    after_help = "For help with a specific command, see: `flir help <command>`."
)]
pub struct Args {
    #[arg(
        short,
        long,
        default_value = ".",
        help = "The directory in which to check or fix lints."
    )]
    dir: String,
    #[arg(
        short,
        long,
        default_value = "false",
        help = "Automatically fix issues detected by the linter."
    )]
    fix: bool,
    #[arg(
        short,
        long,
        default_value = "false",
        help = "Include fixes that may not retain the original intent of the  code."
    )]
    unsafe_fixes: bool,
    #[arg(
        short,
        long,
        default_value = "",
        help = "Names of rules to include, separated by a comma (no spaces)."
    )]
    rules: String,
    #[arg(
        short,
        long,
        default_value = "false",
        help = "Show the time taken by the function."
    )]
    with_timing: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

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
    // let paths = vec![Path::new("demos/foo.R").to_path_buf()];

    let parser_options = RParserOptions::default();
    let config = build_config(&args.rules, args.fix, args.unsafe_fixes, parser_options);

    let diagnostics = check(paths, config)?;

    if !args.fix && !diagnostics.is_empty() {
        for message in &diagnostics {
            println!("{}", message);
        }
    }

    if let Some(start) = start {
        let duration = start.elapsed();
        println!("\nChecked files in: {:?}", duration);
    }

    Ok(())
}
