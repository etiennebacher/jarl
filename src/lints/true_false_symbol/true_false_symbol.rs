use crate::location::Location;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::utils::find_row_col;
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;

pub struct TrueFalseSymbol;

impl Violation for TrueFalseSymbol {
    fn name(&self) -> String {
        "true_false_symbol".to_string()
    }
    fn body(&self) -> String {
        "`T` and `F` can be confused with variable names. Spell `TRUE` and `FALSE` entirely instead.".to_string()
    }
}

impl LintChecker for TrueFalseSymbol {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &[usize], file: &str) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        if ast.kind() == RSyntaxKind::R_IDENTIFIER
            && (ast.text_trimmed() == "T" || ast.text_trimmed() == "F")
        {
            let (row, column) = find_row_col(ast, loc_new_lines);
            let range = ast.text_trimmed_range();
            diagnostics.push(Diagnostic {
                message: TrueFalseSymbol.into(),
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
        diagnostics
    }
}
