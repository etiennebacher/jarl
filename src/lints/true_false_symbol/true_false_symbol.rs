use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;
use anyhow::Result;
use biome_rowan::AstNode;

pub struct TrueFalseSymbol;

/// ## What it does
///
/// Checks for usage of `T` and `F` symbols. If they correspond to the `TRUE`
/// and `FALSE` values, then replace them by that. If they correspond to
/// something else, such as an object or a variable name, then no automatic
/// fixes are applied.
///
/// ## Why is this bad?
///
/// `T` and `F` are not reserved symbols (like `break`) and therefore can be
/// used as variable names. Therefore, it is better for readability to replace
/// them by `TRUE` and `FALSE`.
///
/// It is also recommended to rename objects or parameters named `F` and `T` to
/// avoid confusion.
///
/// ## Example
///
/// ```r
/// x <- T
/// y <- F
/// ```
///
/// Use instead:
/// ```r
/// x <- TRUE
/// y <- FALSE
/// ```
impl Violation for TrueFalseSymbol {
    fn name(&self) -> String {
        "true_false_symbol".to_string()
    }
    fn body(&self) -> String {
        "`T` and `F` can be confused with variable names. Spell `TRUE` and `FALSE` entirely instead.".to_string()
    }
}

impl LintChecker for TrueFalseSymbol {
    fn check(&self, ast: &RSyntaxNode, file: &str) -> Result<Vec<Diagnostic>> {
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

        let range = ast.text_trimmed_range();
        diagnostics.push(Diagnostic::new(
            TrueFalseSymbol,
            file,
            range,
            Fix {
                content: if ast.text_trimmed() == "T" {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                },
                start: range.start().into(),
                end: range.end().into(),
            },
        ));

        Ok(diagnostics)
    }
}
