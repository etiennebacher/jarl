use crate::location::Location;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::utils::{find_row_col, get_args};
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;

pub struct AnyIsNa;

impl Violation for AnyIsNa {
    fn name(&self) -> String {
        "any-na".to_string()
    }
    fn body(&self) -> String {
        "`any(is.na(...))` is inefficient. Use `anyNA(...)` instead.".to_string()
    }
}

impl LintChecker for AnyIsNa {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &[usize], file: &str) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        if ast.kind() != RSyntaxKind::R_CALL {
            return diagnostics;
        }
        let call = ast.first_child().unwrap().text_trimmed();
        if call != "any" {
            return diagnostics;
        }

        get_args(ast).and_then(|args| args.first_child()).map(|y| {
            if y.kind() == RSyntaxKind::R_CALL {
                let fun = y.first_child().unwrap();
                let fun_content = y.children().nth(1).unwrap().first_child().unwrap().text();
                if fun.text_trimmed() == "is.na" && fun.kind() == RSyntaxKind::R_IDENTIFIER {
                    let (row, column) = find_row_col(ast, loc_new_lines);
                    let range = ast.text_trimmed_range();
                    diagnostics.push(Diagnostic {
                        message: AnyIsNa.into(),
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
        diagnostics
    }
}
