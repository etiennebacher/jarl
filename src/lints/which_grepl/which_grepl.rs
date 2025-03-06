use crate::location::Location;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::utils::{find_row_col, get_first_arg};
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;

pub struct WhichGrepl;

impl Violation for WhichGrepl {
    fn name(&self) -> String {
        "which_grepl".to_string()
    }
    fn body(&self) -> String {
        "`grep(pattern, x)` is better than `which(grepl(pattern, x))`.".to_string()
    }
}

impl LintChecker for WhichGrepl {
    fn check(
        &self,
        ast: &RSyntaxNode,
        loc_new_lines: &[usize],
        file: &str,
    ) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        if ast.kind() != RSyntaxKind::R_CALL {
            return Ok(diagnostics);
        }
        let call = ast.first_child().unwrap().text_trimmed();
        if call != "which" {
            return Ok(diagnostics);
        }

        get_first_arg(ast)
            .and_then(|args| args.first_child())
            .map(|y| {
                if y.kind() == RSyntaxKind::R_CALL {
                    let fun = y.first_child().unwrap();
                    let fun_content = y.children().nth(1).unwrap().first_child().unwrap().text();
                    if fun.text_trimmed() == "grepl" && fun.kind() == RSyntaxKind::R_IDENTIFIER {
                        let (row, column) = find_row_col(ast, loc_new_lines);
                        let range = ast.text_trimmed_range();
                        diagnostics.push(Diagnostic {
                            message: WhichGrepl.into(),
                            filename: file.into(),
                            location: Location { row, column },
                            fix: Fix {
                                content: format!("grep({})", fun_content),
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
