use air_r_parser::RParserOptions;
use air_r_syntax::{RLanguage, RSyntaxKind, RSyntaxNode};

use r::lints::*;
use r::message::*;
use r::utils::*;

use rayon::prelude::*;
use std::fs;
use std::path::Path;
use std::time::Instant;

fn main() {
    let start = Instant::now();
    let r_files = vec!["foo2.R"];
    // let r_files = vec!["foo.R", "foo2.R", "foo3.R"];
    let parser_options = RParserOptions::default();
    let messages: Vec<Message> = r_files
        .par_iter()
        .map(|file| {
            let contents = fs::read_to_string(Path::new(file)).expect("couldn't read file");
            let parsed = air_r_parser::parse(contents.as_str(), parser_options);
            let out = &parsed.syntax::<RLanguage>();
            let loc_new_lines = find_new_lines(out);
            check_ast(out, &loc_new_lines, file)
        })
        .flatten()
        .collect();

    for message in messages {
        println!("{}", message);
    }
    let duration = start.elapsed();
    println!("Checked files in: {:?}", duration);
}

fn check_ast(ast: &RSyntaxNode, loc_new_lines: &Vec<usize>, file: &str) -> Vec<Message> {
    let mut messages: Vec<Message> = vec![];

    let linters: Vec<Box<dyn LintChecker>> = vec![
        Box::new(AnyIsNa),
        Box::new(TrueFalseSymbol),
        // Add more lints here as needed
    ];

    for linter in linters {
        messages.extend(linter.check(ast, loc_new_lines, file));
    }

    match ast.kind() {
        RSyntaxKind::R_EXPRESSION_LIST => {
            for child in ast.children() {
                messages.extend(check_ast(&child, loc_new_lines, file));
            }
        }
        RSyntaxKind::R_CALL_ARGUMENTS
        | RSyntaxKind::R_ARGUMENT_LIST
        | RSyntaxKind::R_ARGUMENT
        | RSyntaxKind::R_ROOT => {
            if let Some(x) = &ast.first_child() {
                messages.extend(check_ast(x, loc_new_lines, file))
            }
        }
        RSyntaxKind::R_IDENTIFIER => {
            let fc = &ast.first_child();
            let _has_child = fc.is_some();
            let ns = ast.next_sibling();
            let has_sibling = ns.is_some();
            if has_sibling {
                messages.extend(check_ast(&ns.unwrap(), loc_new_lines, file));
            }
        }
        _ => match &ast.first_child() {
            Some(x) => messages.extend(check_ast(x, loc_new_lines, file)),
            None => {
                let ns = ast.next_sibling();
                let has_sibling = ns.is_some();
                if has_sibling {
                    messages.extend(check_ast(&ns.unwrap(), loc_new_lines, file));
                }
            }
        },
    };

    messages
}
