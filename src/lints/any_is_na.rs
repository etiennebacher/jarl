use crate::location::Location;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::utils::{find_row_col, get_args};
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;

pub struct AnyIsNa;

impl LintChecker for AnyIsNa {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &[usize], file: &str) -> Vec<Message> {
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
                        },
                    })
                };
            }
        });
        messages
    }
}
