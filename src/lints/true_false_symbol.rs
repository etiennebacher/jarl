use crate::location::Location;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::utils::find_row_col;
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;

pub struct TrueFalseSymbol;

impl LintChecker for TrueFalseSymbol {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &[usize], file: &str) -> Vec<Message> {
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
                },
            });
        }
        messages
    }
}
