use crate::message::*;
use crate::utils::{find_row_col, get_args};
use air_r_syntax::{RSyntaxKind, RSyntaxNode};

pub trait LintChecker {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &Vec<usize>, file: &str) -> Vec<Message>;
}

pub struct AnyIsNa;

impl LintChecker for AnyIsNa {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &Vec<usize>, file: &str) -> Vec<Message> {
        let mut messages = vec![];
        if ast.kind() == RSyntaxKind::R_CALL {
            let call = ast.first_child().unwrap().text_trimmed();
            if call == "any" {
                let args = get_args(ast);
                if let Some(x) = args {
                    let first_arg = x.first_child().unwrap().first_child().unwrap();
                    if first_arg.text_trimmed() == "is.na"
                        && first_arg.kind() == RSyntaxKind::R_IDENTIFIER
                    {
                        let (row, column) = find_row_col(ast, loc_new_lines);
                        messages.push(Message::AnyIsNa {
                            filename: file.into(),
                            location: Location { row, column },
                        });
                    }
                }
            }
        }
        messages
    }
}

pub struct TrueFalseSymbol;

impl LintChecker for TrueFalseSymbol {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &Vec<usize>, file: &str) -> Vec<Message> {
        let mut messages = vec![];
        if ast.kind() == RSyntaxKind::R_IDENTIFIER {
            if ast.text_trimmed() == "T" || ast.text_trimmed() == "F" {
                let (row, column) = find_row_col(ast, loc_new_lines);
                messages.push(Message::TrueFalseSymbol {
                    filename: file.into(),
                    location: Location { row, column },
                });
            }
        }
        messages
    }
}

// Add other lints here as needed...
