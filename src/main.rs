use air_r_parser::RParserOptions;

use flint::check_ast::*;
use flint::fix::*;
use flint::message::*;

use clap::{arg, Parser};
use flint::semantic_model;
use flint::SemanticModelOptions;
use rayon::prelude::*;
use std::fs;
use std::path::Path;
// use std::time::Instant;
use walkdir::WalkDir;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(
    author,
    name = "flint",
    about = "Flint: Find and Fix Lints in R Code",
    after_help = "For help with a specific command, see: `flint help <command>`."
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
}

/// This is my first rust crate
fn main() {
    // let start = Instant::now();
    // let args = Args::parse();

    // let r_files = WalkDir::new(args.dir)
    //     .into_iter()
    //     .filter_map(Result::ok)
    //     .filter(|e| e.file_type().is_file())
    //     .filter(|e| {
    //         e.path().extension() == Some(std::ffi::OsStr::new("R"))
    //             || e.path().extension() == Some(std::ffi::OsStr::new("r"))
    //     })
    //     .map(|e| e.path().to_path_buf())
    //     .collect::<Vec<_>>();

    let r_files = vec![Path::new("demo/foo.R").to_path_buf()];
    let parser_options = RParserOptions::default();
    let contents = fs::read_to_string(Path::new(&r_files[0])).expect("Invalid file");
    let parsed = air_r_parser::parse(&contents, parser_options);
    let root = &parsed.tree();

    let semantic = semantic_model(root, SemanticModelOptions::default());

    println!("semantic model: {:?}", semantic);

    // let parser_options = RParserOptions::default();
    // let messages: Vec<Message> = r_files
    //     .par_iter()
    //     // TODO: this only ignores files where there was an error, it doesn't
    //     // return the error messages
    //     .filter_map(|file| {
    //         let mut checks: Vec<Message>;
    //         let mut has_skipped_fixes = true;
    //         loop {
    //             let contents = fs::read_to_string(Path::new(file)).expect("Invalid file");
    //             checks = get_checks(&contents, file, parser_options).unwrap();
    //             if !has_skipped_fixes || !args.fix {
    //                 break;
    //             }
    //             let (new_has_skipped_fixes, fixed_text) = apply_fixes(&checks, &contents);
    //             has_skipped_fixes = new_has_skipped_fixes;
    //             let _ = fs::write(file, fixed_text);
    //         }
    //         Some(checks)
    //     })
    //     .flatten()
    //     .collect();

    // if !args.fix {
    //     for message in messages {
    //         println!("{}", message);
    //     }
    // }
    // let duration = start.elapsed();
    // println!("Checked files in: {:?}", duration);
}
