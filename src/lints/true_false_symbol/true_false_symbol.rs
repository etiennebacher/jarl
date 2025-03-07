use crate::location::Location;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::utils::find_row_col;
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;
use anyhow::Result;
use biome_rowan::AstNode;

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
    fn check(
        &self,
        ast: &RSyntaxNode,
        loc_new_lines: &[usize],
        file: &str,
    ) -> Result<Vec<Diagnostic>> {
        let mut diagnostics: Vec<Diagnostic> = vec![];
        if ast.kind() != RSyntaxKind::R_IDENTIFIER
            || (ast.text_trimmed() != "T" && ast.text_trimmed() != "F")
        {
            return Ok(diagnostics);
        }

        // Allow T(), F()
        let is_function_name = ast
            .parent()
            .map(|x| x.kind() == RSyntaxKind::R_CALL)
            .unwrap_or(false);

        // Allow df$T, df$F
        let is_element_name = ast
            .parent()
            .map(|x| x.kind() == RSyntaxKind::R_EXTRACT_EXPRESSION)
            .unwrap_or(false);

        // Allow A ~ T
        let is_in_formula = ast
            .parent()
            .map(|x| {
                let bin_expr = RBinaryExpression::cast(x.clone());
                if bin_expr.is_some() {
                    let RBinaryExpressionFields { left: _, operator, right: _ } =
                        bin_expr.unwrap().as_fields();

                    let operator = operator.unwrap();
                    operator.kind() == RSyntaxKind::TILDE
                } else {
                    false
                }
            })
            .unwrap_or(false);

        if is_function_name || is_element_name || is_in_formula {
            return Ok(diagnostics);
        }

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

        Ok(diagnostics)
    }
}
