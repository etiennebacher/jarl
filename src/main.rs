use air_r_parser::RParserOptions;
use air_r_syntax::RLanguage;

use flint::check_ast::*;
use flint::location::Location;
use flint::message::*;
use flint::utils::*;

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
    #[arg(short, long, default_value = "true")]
    fix: bool,
}

fn main() {
    let start = Instant::now();
    let args = Args::parse();

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

    let r_files = vec![Path::new("demo/foo.R")];

    let parser_options = RParserOptions::default();
    let messages: Vec<Message> = r_files
        .par_iter()
        .map(|file| {
            let contents = fs::read_to_string(Path::new(file)).expect("couldn't read file");
            let parsed = air_r_parser::parse(contents.as_str(), parser_options);
            let out = &parsed.syntax::<RLanguage>();
            let loc_new_lines = find_new_lines(out);
            check_ast(out, &loc_new_lines, file.to_str().unwrap())
        })
        .flatten()
        .collect();

    if args.fix {
        let contents = fs::read_to_string(Path::new("demo/foo.R")).expect("couldn't read file");
        let out = apply_fixes(messages, &contents);
        println!("{}", out);
    } else {
        for message in messages {
            println!("{}", message);
        }
    }
    let duration = start.elapsed();
    println!("Checked files in: {:?}", duration);
}

fn apply_fixes(fixes: Vec<Message>, contents: &str) -> String {
    let lines: Vec<&str> = contents.lines().collect();
    let fixes = fixes
        .iter()
        .map(|msg| match msg {
            Message::AnyDuplicated { fix, .. }
            | Message::AnyIsNa { fix, .. }
            | Message::TrueFalseSymbol { fix, .. } => fix,
        })
        .collect::<Vec<_>>();
    let mut output = "".to_string();
    let mut last_pos = Location::new(0, 0);

    for fix in fixes {
        // Best-effort approach: if this fix overlaps with a fix we've already applied, skip it.
        if last_pos > fix.start {
            continue;
        }

        if fix.start.row() > last_pos.row() {
            if last_pos.row() > 0 || last_pos.column() > 0 {
                output.push_str(&lines[last_pos.row() - 1][last_pos.column() - 1..]);
                output.push('\n');
            }
            for line in &lines[last_pos.row()..fix.start.row()] {
                output.push_str(line);
                output.push('\n');
            }
            output.push_str(&lines[fix.start.row() - 1][..fix.start.column() - 1]);
            output.push_str(&fix.content);
        } else {
            output.push_str(&lines[last_pos.row()][last_pos.column()..fix.start.column() - 1]);
            output.push_str(&fix.content);
        }

        last_pos = fix.end;
        // fix.applied = true;
    }
    // println!("output: {:?}", output);
    // println!("lines: {:?}", lines);
    // println!("last_pos: {:?}", last_pos);
    // println!("last_pos_row: {:?}", last_pos.row());
    // println!("last_pos_column: {:?}", last_pos.column());

    // if last_pos.row() > 0 || last_pos.column() > 0 {
    //     output.push_str(&lines[last_pos.row() - 1][last_pos.column() - 1..]);
    //     output.push('\n');
    // }
    // for line in &lines[last_pos.row()..] {
    //     output.push_str(line);
    //     output.push('\n');
    // }

    output
}
