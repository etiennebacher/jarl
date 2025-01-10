use air_r_syntax::{RSyntaxKind, RSyntaxNode};

use crate::lints::*;
use crate::message::*;

pub fn check_ast(ast: &RSyntaxNode, loc_new_lines: &[usize], file: &str) -> Vec<Message> {
    let mut messages: Vec<Message> = vec![];

    // println!("{:?}", ast);
    // println!("{:?}", ast.text());

    let linters: Vec<Box<dyn LintChecker>> = vec![
        Box::new(AnyIsNa),
        Box::new(TrueFalseSymbol),
        Box::new(AnyDuplicated),
        Box::new(ClassEquals),
    ];

    for linter in linters {
        messages.extend(linter.check(ast, loc_new_lines, file));
    }

    match ast.kind() {
        RSyntaxKind::R_EXPRESSION_LIST
        | RSyntaxKind::R_FUNCTION_DEFINITION
        | RSyntaxKind::R_BINARY_EXPRESSION
        | RSyntaxKind::R_FOR_STATEMENT => {
            for child in ast.children() {
                messages.extend(check_ast(&child, loc_new_lines, file));
            }
        }
        RSyntaxKind::R_CALL_ARGUMENTS
        | RSyntaxKind::R_ARGUMENT_LIST
        | RSyntaxKind::R_ARGUMENT
        | RSyntaxKind::R_ROOT
        | RSyntaxKind::R_WHILE_STATEMENT
        | RSyntaxKind::R_IF_STATEMENT => {
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
