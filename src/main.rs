use air_r_parser::RParserOptions;
use air_r_syntax::RLanguage;

use flint::check_ast::*;
use flint::message::*;
use flint::utils::*;
use walkdir::WalkDir;

use clap::{arg, Parser};
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use std::time::Instant;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = ".")]
    dir: String,
    #[arg(short, long, default_value = "false")]
    fix: bool,
}

fn main() {
    let start = Instant::now();
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

    // let r_files = vec![Path::new("demo/foo.R")];

    let parser_options = RParserOptions::default();
    let messages: Vec<Message> = r_files
        .par_iter()
        .map(|file| {
            let contents = fs::read_to_string(Path::new(file)).expect("couldn't read file");
            let parsed = air_r_parser::parse(contents.as_str(), parser_options);
            let out = &parsed.syntax::<RLanguage>();
            let loc_new_lines = find_new_lines(out);
            let checks = check_ast(out, &loc_new_lines, file.to_str().unwrap());
            if args.fix {
                let out = apply_fixes(&checks, &contents);
                let _ = fs::write(file, out);
            }
            checks
        })
        .flatten()
        .collect();

    if !args.fix {
        for message in messages {
            println!("{}", message);
        }
    }
    let duration = start.elapsed();
    println!("Checked files in: {:?}", duration);
}

fn apply_fixes(fixes: &Vec<Message>, contents: &str) -> String {
    let fixes = fixes
        .iter()
        .map(|msg| match msg {
            Message::AnyDuplicated { fix, .. }
            | Message::AnyIsNa { fix, .. }
            | Message::TrueFalseSymbol { fix, .. } => fix,
        })
        .collect::<Vec<_>>();
    let old_content = contents;
    let mut new_content = old_content.to_string();

    let offset = 0_usize;

    let old_length: i32 = old_content.chars().count().try_into().unwrap();
    let mut new_length: i32 = old_length;

    for fix in fixes {
        let mut start = (fix.start - offset) as i32;
        let mut end = (fix.end - offset) as i32;

        // println!("original start: {}", start);
        // println!("original end: {}", end);
        // println!("old_length: {}", old_length);
        // println!("new_length: {}", new_length);

        let diff_length = new_length - old_length;
        // println!("diff_length: {}", diff_length);

        if diff_length != 0 {
            start += diff_length;
            end += diff_length;
        }

        // println!("new start: {}", start);
        // println!("new end: {}\n", end);

        let start_usize = start as usize;
        let end_usize = end as usize;

        new_content.replace_range(start_usize..end_usize, &fix.content);

        new_length = new_content.chars().count().try_into().unwrap();
    }

    new_content.to_string()
}
