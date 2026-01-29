use crate::diagnostic::*;
use air_r_syntax::*;
use biome_rowan::{AstNode, Text};

pub struct ForLoopIndex;

/// ## What it does
///
/// Checks whether the index symbol in a for loop is already used anywhere in
/// the sequence of the same for loop.
///
/// ## Why is this bad?
///
/// `for (x in x)` or `for (x in foo(x))` are confusing to read and can lead
/// to errors.
///
/// ## Example
///
/// ```r
/// x <- c(1, 2, 3)
/// for (x in x) {
///   x + 1
/// }
/// ```
///
/// Use instead:
/// ```r
/// x <- c(1, 2, 3)
/// for (xi in x) {
///   xi + 1
/// }
/// ```
impl Violation for ForLoopIndex {
    fn name(&self) -> String {
        "for_loop_index".to_string()
    }
    fn body(&self) -> String {
        "Don't re-use any sequence symbols as the index symbol in a for loop.".to_string()
    }
}

pub fn for_loop_index(ast: &RForStatement) -> anyhow::Result<Option<Diagnostic>> {
    let RForStatementFields { variable, sequence, .. } = ast.as_fields();

    let variable_text = variable?.to_trimmed_text();
    let sequence = sequence?;

    if contains_identifier(&sequence, &variable_text)? {
        let range_start = ast.variable()?.range().start();
        let range_end = ast.sequence()?.range().end();
        let range = TextRange::new(range_start, range_end);
        let diagnostic = Diagnostic::new(ForLoopIndex, range, Fix::empty());
        Ok(Some(diagnostic))
    } else {
        Ok(None)
    }
}

fn contains_identifier(expr: &AnyRExpression, target: &str) -> anyhow::Result<bool> {
    let out = match expr {
        AnyRExpression::RIdentifier(ident) => ident.to_trimmed_text() == target,
        AnyRExpression::RCall(call) => {
            let arguments = call.arguments()?.items();

            let arg_names: Vec<Text> = arguments
                .clone()
                .into_iter()
                .filter_map(|x| {
                    let expr = x.ok()?; // Convert Result to Option
                    let name_clause = expr.as_fields().name_clause?;
                    let name = name_clause.name().unwrap();
                    Some(name.to_trimmed_text())
                })
                .collect();

            if arg_names.iter().any(|x| *x == target) {
                return Ok(true);
            }

            let arg_values: Vec<AnyRExpression> = arguments
                .into_iter()
                .filter_map(|x| x.unwrap().as_fields().value)
                .collect();

            for expr in arg_values {
                if contains_identifier(&expr, target)? {
                    return Ok(true);
                }
            }
            false
        }
        _ => false,
    };

    Ok(out)
}
