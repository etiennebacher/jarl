use crate::message::*;
use crate::trait_lint_checker::LintChecker;
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
    fn check(&self, ast: &AnyRExpression, file: &str) -> Result<Vec<Diagnostic>> {
        let mut diagnostics: Vec<Diagnostic> = vec![];
        let ast = if let Some(ast) = ast.as_r_identifier() {
            ast
        } else {
            return Ok(diagnostics);
        };
        let token = ast.name_token().unwrap();
        let name = token.text_trimmed();
        if name != "T" && name != "F" {
            return Ok(diagnostics);
        }

        // Allow T(), F()
        let is_function_name = ast.parent::<RCall>().is_some();

        // Allow df$T, df$F
        let is_element_name = ast.parent::<RExtractExpression>().is_some();

        // Allow A ~ T
        let is_in_formula = ast
            .parent::<RBinaryExpression>()
            .map(|x| {
                let operator = x.operator().unwrap();
                operator.kind() == RSyntaxKind::TILDE
            })
            .unwrap_or(false);

        if is_function_name || is_element_name || is_in_formula {
            return Ok(diagnostics);
        }

        let range = ast.clone().into_syntax().text_trimmed_range();
        diagnostics.push(Diagnostic::new(
            TrueFalseSymbol,
            file,
            range,
            Fix {
                content: if ast.clone().into_syntax().text_trimmed() == "T" {
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
