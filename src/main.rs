use air_r_parser::RParserOptions;

use flir::check_ast::*;
use flir::fix::*;
use flir::message::*;
use flir::semantic_analysis::check_unused_vars::*;
use flir::utils::parse_rules;

use clap::{arg, Parser};
use flir::semantic_model;
use flir::SemanticModelOptions;
use rayon::prelude::*;
use std::fs;
use std::path::Path;
// use std::time::Instant;
use colored::Colorize;
use walkdir::WalkDir;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(
    author,
    name = "flir",
    about = "Flint: Find and Fix Lints in R Code",
    after_help = "For help with a specific command, see: `flir help <command>`."
)]
struct Args {
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
        default_value = "",
        help = "Names of rules to include, separated by a comma (no spaces)"
    )]
    rules: String,
}

/// This is my first rust crate
fn main() {
    // let start = Instant::now();
    let args = Args::parse();

    let r_files = WalkDir::new(args.dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path().extension() == Some(std::ffi::OsStr::new("R"))
                || e.path().extension() == Some(std::ffi::OsStr::new("r"))
        })
        .map(|e| e.path().to_path_buf())
        .collect::<Vec<_>>();

    let rules = parse_rules(&args.rules);

    // let r_files = vec![Path::new("demo/foo.R").to_path_buf()];

    let parser_options = RParserOptions::default();
    let _: Vec<Diagnostic> = r_files
        .par_iter()
        // TODO: this only ignores files where there was an error, it doesn't
        // return the error diagnostics
        .filter_map(|file| {
            let mut checks: Vec<Diagnostic>;
            let mut has_skipped_fixes = true;
            loop {
                let contents = fs::read_to_string(Path::new(file)).expect("Invalid file");

                checks = get_checks(&contents, file, parser_options, rules.clone()).unwrap();
                if !has_skipped_fixes || !args.fix {
                    break;
                }
                let (new_has_skipped_fixes, fixed_text) = apply_fixes(&checks, &contents);
                has_skipped_fixes = new_has_skipped_fixes;
                let _ = fs::write(file, fixed_text);
            }

            if !args.fix && &checks.len() > &0usize {
                // println!("{}", file.to_str().unwrap().blue().bold());
                for message in &checks {
                    println!("{}", message);
                }
            }
            Some(checks)
        })
        .flatten()
        .collect();
    // let duration = start.elapsed();
    // println!("Checked files in: {:?}", duration);
}
