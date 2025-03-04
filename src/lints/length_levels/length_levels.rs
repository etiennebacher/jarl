use crate::location::Location;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::utils::{find_row_col, get_first_arg};
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;

pub struct LengthLevels;

impl Violation for LengthLevels {
    fn name(&self) -> String {
        "length_levels".to_string()
    }
    fn body(&self) -> String {
        "Use `nlevels(...)` instead of `length(levels(...))`.".to_string()
    }
}

impl LintChecker for LengthLevels {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &[usize], file: &str) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        if ast.kind() != RSyntaxKind::R_CALL {
            return diagnostics;
        }
        let call = ast.first_child().unwrap().text_trimmed();
        if call != "length" {
            return diagnostics;
        }

        get_first_arg(ast).and_then(|args| args.first_child()).map(|y| {
            if y.kind() == RSyntaxKind::R_CALL {
                let fun = y.first_child().unwrap();
                let fun_content = y.children().nth(1).unwrap().first_child().unwrap().text();
                if fun.text_trimmed() == "levels" && fun.kind() == RSyntaxKind::R_IDENTIFIER {
                    let (row, column) = find_row_col(ast, loc_new_lines);
                    let range = ast.text_trimmed_range();
                    diagnostics.push(Diagnostic {
                        message: LengthLevels.into(),
                        filename: file.into(),
                        location: Location { row, column },
                        fix: Fix {
                            content: format!("nlevels({})", fun_content),
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
