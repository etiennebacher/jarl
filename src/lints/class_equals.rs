use crate::location::Location;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::utils::{find_row_col, get_args};
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct ClassEquals;

impl LintChecker for ClassEquals {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &[usize], file: &str) -> Vec<Message> {
        let mut messages = vec![];
        let bin_expr = RBinaryExpression::cast(ast.clone());
        if bin_expr.is_none() {
            return messages;
        }

        println!(
            "PARENT: {:?}",
            ast.ancestors().map(|x| x.kind()).collect::<Vec<_>>()
        );

        let RBinaryExpressionFields { left: _, operator, right: _ } = bin_expr.unwrap().as_fields();

        let operator = operator.unwrap();

        if operator.kind() != RSyntaxKind::EQUAL2 && operator.kind() != RSyntaxKind::NOT_EQUAL {
            return messages;
        };

        let mut children = ast.children();
        let lhs = children.next().unwrap();
        let rhs = children.next().unwrap();

        if let Some(fun) = lhs.first_child() {
            if fun.text_trimmed() != "class" {
                return messages;
            }
        } else {
            return messages;
        }

        if rhs.kind() != RSyntaxKind::R_STRING_VALUE {
            return messages;
        }

        let fun_content = get_args(&lhs).and_then(|x| Some(x.text_trimmed()));

        let (row, column) = find_row_col(ast, loc_new_lines);
        let range = ast.text_trimmed_range();
        messages.push(Message::ClassEquals {
            filename: file.into(),
            location: Location { row, column },
            fix: Fix {
                content: format!("inherits({}, {})", fun_content.unwrap(), rhs.text_trimmed()),
                start: range.start().into(),
                end: range.end().into(),
            },
        });
        messages
    }
}
