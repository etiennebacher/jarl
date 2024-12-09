use crate::message::*;
use crate::utils::{find_row_col, get_args};
use air_r_syntax::{RSyntaxKind, RSyntaxNode};

pub trait LintChecker {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &Vec<usize>, file: &str) -> Vec<Message>;
}

pub struct AnyIsNa;
pub struct AnyDuplicated;
pub struct TrueFalseSymbol;

impl LintChecker for AnyIsNa {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &Vec<usize>, file: &str) -> Vec<Message> {
        let mut messages = vec![];
        if ast.kind() != RSyntaxKind::R_CALL {
            return messages;
        }
        let call = ast.first_child().unwrap().text_trimmed();
        if call != "any" {
            return messages;
        }

        get_args(ast)
            .and_then(|args| args.first_child())
            .and_then(|y| y.first_child())
            .filter(|first_arg| {
                first_arg.text_trimmed() == "is.na" && first_arg.kind() == RSyntaxKind::R_IDENTIFIER
            })
            .map(|_| {
                let (row, column) = find_row_col(ast, loc_new_lines);
                messages.push(Message::AnyIsNa {
                    filename: file.into(),
                    location: Location { row, column },
                });
            });

        messages
    }
}

impl LintChecker for AnyDuplicated {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &Vec<usize>, file: &str) -> Vec<Message> {
        let mut messages = vec![];
        if ast.kind() != RSyntaxKind::R_CALL {
            return messages;
        }
        let call = ast.first_child().unwrap().text_trimmed();
        if call != "any" {
            return messages;
        }

        get_args(ast)
            .and_then(|args| args.first_child())
            .and_then(|y| y.first_child())
            .filter(|first_arg| {
                first_arg.text_trimmed() == "duplicated"
                    && first_arg.kind() == RSyntaxKind::R_IDENTIFIER
            })
            .map(|_| {
                let (row, column) = find_row_col(ast, loc_new_lines);
                messages.push(Message::AnyIsNa {
                    filename: file.into(),
                    location: Location { row, column },
                });
            });
        messages
    }
}

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
