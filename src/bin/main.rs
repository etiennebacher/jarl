use air_r_parser::{parse, RParserOptions};
use air_r_syntax::{RLanguage, RSyntaxKind, RSyntaxNode, TextSize};

use biome_rowan::SyntaxNode;
use r::message::*;
use r::object::Expr;
use r::parser::*;
use r::session::SessionParserConfig;
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufReader, Read};
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
            let file = File::open(Path::new(file)).unwrap();
            let mut buf_reader = BufReader::new(file);
            let mut contents = String::new();
            buf_reader.read_to_string(&mut contents).unwrap();
            let parsed = air_r_parser::parse(contents.as_str(), parser_options);
            let out = &parsed.syntax::<RLanguage>();
            let loc_new_lines = find_new_lines(&out);
            let msg = check_ast(&out, loc_new_lines);
            msg
        })
        .flatten()
        .collect();

    for message in messages {
        println!("{}", message);
    }
    let duration = start.elapsed();
    println!("Checked files in: {:?}", duration);
}

fn check_ast(ast: &RSyntaxNode, loc_new_lines: Vec<usize>) -> Vec<Message> {
    let mut messages: Vec<Message> = vec![];
    // println!("{}", ast);
    println!("{:?}", ast.kind());
    let _ = match ast.kind() {
        RSyntaxKind::R_EXPRESSION_LIST
        | RSyntaxKind::R_CALL
        | RSyntaxKind::R_CALL_ARGUMENTS
        | RSyntaxKind::R_ARGUMENT_LIST
        | RSyntaxKind::R_ARGUMENT
        | RSyntaxKind::R_ROOT => match &ast.first_child() {
            Some(x) => messages.extend(check_ast(x, loc_new_lines)),
            None => println!("foo1"),
        },
        RSyntaxKind::R_IDENTIFIER => {
            if ast.text_trimmed() == "T" {
                let (row, column) = find_row_col(ast, &loc_new_lines);
                messages.push(Message::TrueFalseSymbol {
                    filename: "foobar".into(),
                    location: Location { row, column },
                });
            }
            let fc = &ast.first_child();
            let has_child = fc.is_some();
            let ns = ast.next_sibling();
            let has_sibling = ns.is_some();
            if has_sibling {
                messages.extend(check_ast(&ns.unwrap(), loc_new_lines));
            } else {
                println!("foo2")
            }
        }
        _ => match &ast.first_child() {
            Some(x) => messages.extend(check_ast(x, loc_new_lines)),
            None => {
                let ns = ast.next_sibling();
                let has_sibling = ns.is_some();
                if has_sibling {
                    messages.extend(check_ast(&ns.unwrap(), loc_new_lines));
                } else {
                    println!("foo3")
                }
            }
        },
    };
    messages
}

fn find_new_lines(ast: &RSyntaxNode) -> Vec<usize> {
    ast.first_child()
        .unwrap()
        .text()
        .to_string()
        .match_indices("\n")
        .map(|x| x.0)
        .collect::<Vec<usize>>()
}

fn find_row_col(ast: &RSyntaxNode, loc_new_lines: &Vec<usize>) -> (usize, usize) {
    let locs = ast.text_range();
    let start: usize = locs.start().into();
    let new_lines_before = loc_new_lines
        .iter()
        .filter(|x| *x < &start)
        .collect::<Vec<&usize>>();
    let n_new_lines = new_lines_before.len();
    let last_new_line = new_lines_before.last().unwrap();

    let row = start - **last_new_line + 1;
    let col = n_new_lines + 1;
    (row, col)
}
