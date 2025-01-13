use crate::location::Location;
use crate::message::*;
use crate::utils::{find_row_col, get_args};
use air_r_syntax::*;

pub trait LintChecker {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &[usize], file: &str) -> Vec<Message>;
}

pub struct AnyIsNa;
pub struct AnyDuplicated;
pub struct ClassEquals;
pub struct EqualsNa;
pub struct TrueFalseSymbol;

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

impl LintChecker for AnyDuplicated {
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
                        },
                    })
                };
            }
        });
        messages
    }
}

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

impl LintChecker for ClassEquals {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &[usize], file: &str) -> Vec<Message> {
        let mut messages = vec![];
        if ast.kind() != RSyntaxKind::R_BINARY_EXPRESSION {
            return messages;
        }

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

impl LintChecker for EqualsNa {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &[usize], file: &str) -> Vec<Message> {
        let mut messages = vec![];

        if ast.kind() != RSyntaxKind::R_BINARY_EXPRESSION {
            return messages;
        }

        let RBinaryExpressionFields { left, operator, right } =
            unsafe { RBinaryExpression::new_unchecked(ast.clone()).as_fields() };

        let left = left.unwrap();
        let operator = operator.unwrap();
        let right = right.unwrap();

        if operator.kind() != RSyntaxKind::EQUAL2 && operator.kind() != RSyntaxKind::NOT_EQUAL {
            return messages;
        };

        let na_values = vec![
            "NA",
            "NA_character_",
            "NA_integer_",
            "NA_real_",
            "NA_logical_",
            "NA_complex_",
        ];

        let left_is_na = na_values.contains(&left.to_string().trim());
        let right_is_na = na_values.contains(&right.to_string().trim());

        // If NA is quoted in text, then quotation marks are escaped and this
        // is false.
        if (left_is_na && right_is_na) || (!left_is_na && !right_is_na) {
            return messages;
        }
        let (row, column) = find_row_col(ast, loc_new_lines);
        let range = ast.text_trimmed_range();

        let replacement = if left_is_na {
            right.to_string().trim().to_string()
        } else {
            left.to_string().trim().to_string()
        };

        match operator.kind() {
            RSyntaxKind::EQUAL2 => {
                messages.push(Message::EqualsNa {
                    filename: file.into(),
                    location: Location { row, column },
                    fix: Fix {
                        content: format!("is.na({})", replacement),
                        start: range.start().into(),
                        end: range.end().into(),
                    },
                });
            }
            RSyntaxKind::NOT_EQUAL => {
                messages.push(Message::EqualsNa {
                    filename: file.into(),
                    location: Location { row, column },
                    fix: Fix {
                        content: format!("!is.na({})", replacement),
                        start: range.start().into(),
                        end: range.end().into(),
                    },
                });
            }
            _ => unreachable!("This case is an early return"),
        };

        messages
    }
}
