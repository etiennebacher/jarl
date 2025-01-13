use air_r_parser::RParserOptions;
use air_r_syntax::RBinaryExpression;
use air_r_syntax::{RSyntaxKind, RSyntaxNode};

use crate::lints::*;
use crate::message::*;
use crate::utils::*;
use anyhow::Result;
use std::path::PathBuf;

pub fn get_checks(
    contents: &String,
    file: &PathBuf,
    parser_options: RParserOptions,
) -> Result<Vec<Message>> {
    let parsed = air_r_parser::parse(contents.as_str(), parser_options);
    let syntax = &parsed.syntax();
    let loc_new_lines = find_new_lines(syntax)?;
    Ok(check_ast(syntax, &loc_new_lines, file.to_str().unwrap()))
}

pub fn check_ast(ast: &RSyntaxNode, loc_new_lines: &[usize], file: &str) -> Vec<Message> {
    let mut messages: Vec<Message> = vec![];

    // println!("{:?}", ast.text());

    // if ast.kind() == RSyntaxKind::R_BINARY_EXPRESSION {
    //     unsafe {
    //         println!(
    //             "AST node: {:?}",
    //             RBinaryExpression::new_unchecked(ast.clone())
    //         )
    //     }
    // };

    // println!("{:?}", ast.kind());
    // println!("Text: {:?}", ast.text_trimmed());
    // println!(
    //     "Children: {:?}",
    //     ast.children().map(|x| x.kind()).collect::<Vec<_>>()
    // );

    let linters: Vec<Box<dyn LintChecker>> = vec![
        Box::new(AnyIsNa),
        Box::new(TrueFalseSymbol),
        Box::new(AnyDuplicated),
        Box::new(ClassEquals),
        Box::new(EqualsNa),
    ];

    for linter in linters {
        messages.extend(linter.check(ast, loc_new_lines, file));
    }

    match ast.kind() {
        RSyntaxKind::R_EXPRESSION_LIST
        | RSyntaxKind::R_FUNCTION_DEFINITION
        | RSyntaxKind::R_CALL_ARGUMENTS
        | RSyntaxKind::R_SUBSET
        | RSyntaxKind::R_SUBSET2
        | RSyntaxKind::R_PARAMETERS
        | RSyntaxKind::R_PARAMETER
        | RSyntaxKind::R_ARGUMENT_LIST
        | RSyntaxKind::R_ARGUMENT
        | RSyntaxKind::R_BRACED_EXPRESSIONS
        | RSyntaxKind::R_ROOT
        | RSyntaxKind::R_REPEAT_STATEMENT
        | RSyntaxKind::R_UNARY_EXPRESSION
        | RSyntaxKind::R_BINARY_EXPRESSION
        | RSyntaxKind::R_PARENTHESIZED_EXPRESSION
        | RSyntaxKind::R_EXTRACT_EXPRESSION
        | RSyntaxKind::R_NAMESPACE_EXPRESSION
        | RSyntaxKind::R_NA_EXPRESSION
        | RSyntaxKind::R_FOR_STATEMENT
        | RSyntaxKind::R_WHILE_STATEMENT
        | RSyntaxKind::R_IF_STATEMENT => {
            for child in ast.children() {
                messages.extend(check_ast(&child, loc_new_lines, file));
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
            Some(_) => {
                for child in ast.children() {
                    messages.extend(check_ast(&child, loc_new_lines, file));
                }
            }
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
