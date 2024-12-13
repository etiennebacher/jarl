use crate::location::Location;
use crate::message::*;
use crate::utils::{find_row_col, get_args};
use air_r_syntax::{RSyntaxKind, RSyntaxNode, TextSize};

pub trait LintChecker {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &[u32], file: &str) -> Vec<Message>;
}

pub struct AnyIsNa;
pub struct AnyDuplicated;
pub struct TrueFalseSymbol;

impl LintChecker for AnyIsNa {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &[u32], file: &str) -> Vec<Message> {
        let mut messages = vec![];
        if ast.kind() != RSyntaxKind::R_CALL {
            return messages;
        }
        let call = ast.first_child().unwrap().text_trimmed();
        if call != "any" {
            return messages;
        }

        get_args(ast).and_then(|args| args.first_child()).map(|y| {
            if y.kind() == RSyntaxKind::R_CALL {
                let fun = y.first_child().unwrap();
                let fun_content = y.children().nth(1).unwrap().first_child().unwrap().text();
                if fun.text_trimmed() == "is.na" && fun.kind() == RSyntaxKind::R_IDENTIFIER {
                    let (row, column) = find_row_col(ast, loc_new_lines);
                    let range = ast.text_trimmed_range();
                    messages.push(Message::AnyIsNa {
                        filename: file.into(),
                        location: Location { row, column },
                        fix: Fix {
                            content: format!("anyNA({})", fun_content),
                            start: range.start().into(),
                            end: range.end().into(),
                            applied: false,
                        },
                    })
                };
            }
        });
        messages
    }
}

impl LintChecker for AnyDuplicated {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &[u32], file: &str) -> Vec<Message> {
        let mut messages = vec![];
        if ast.kind() != RSyntaxKind::R_CALL {
            return messages;
        }
        let call = ast.first_child().unwrap().text_trimmed();
        if call != "any" {
            return messages;
        }

        get_args(ast).and_then(|args| args.first_child()).map(|y| {
            if y.kind() == RSyntaxKind::R_CALL {
                let fun = y.first_child().unwrap();
                let fun_content = y.children().nth(1).unwrap().first_child().unwrap().text();
                if fun.text_trimmed() == "duplicated" && fun.kind() == RSyntaxKind::R_IDENTIFIER {
                    let (row, column) = find_row_col(ast, loc_new_lines);
                    let range = ast.text_trimmed_range();
                    messages.push(Message::AnyDuplicated {
                        filename: file.into(),
                        location: Location { row, column },
                        fix: Fix {
                            content: format!("anyDuplicated({}) > 0", fun_content),
                            start: range.start().into(),
                            end: range.end().into(),
                            applied: false,
                        },
                    })
                };
            }
        });
        messages
    }
}

impl LintChecker for TrueFalseSymbol {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &[u32], file: &str) -> Vec<Message> {
        let mut messages = vec![];
        if ast.kind() == RSyntaxKind::R_IDENTIFIER
            && (ast.text_trimmed() == "T" || ast.text_trimmed() == "F")
        {
            let (row, column) = find_row_col(ast, loc_new_lines);
            let range = ast.text_trimmed_range();
            messages.push(Message::TrueFalseSymbol {
                filename: file.into(),
                location: Location { row, column },
                fix: Fix {
                    content: if ast.text_trimmed() == "T" {
                        "TRUE".to_string()
                    } else {
                        "FALSE".to_string()
                    },
                    start: range.start().into(),
                    end: range.end().into(),
                    applied: false,
                },
            });
        }
        messages
    }
}
