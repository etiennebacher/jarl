use crate::{diagnostic::*, utils::get_function_name};
use air_r_syntax::RSyntaxKind::*;
use air_r_syntax::*;
use anyhow::Context;
use biome_rowan::AstNode;

pub struct LengthTest;

/// ## What it does
///
/// Checks for usage of `length(... == some_val)` and replaces it with
/// `length(...) == some_val`.
///
/// ## Why is this bad?
///
/// This is very likely a mistake since computing the length of the output of
/// `==` is the same as computing the length of the inputs.
///
/// ## Example
///
/// ```r
/// x <- 1:3
/// length(x == 1)
/// ```
///
/// Use instead:
/// ```r
/// x <- 1:3
/// length(x) == 1
/// ```
impl Violation for LengthTest {
    fn name(&self) -> String {
        "length_test".to_string()
    }
    fn body(&self) -> String {
        "Checking the length of a logical vector is likely a mistake".to_string()
    }
}

pub fn length_test(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let RCallFields { function, arguments } = ast.as_fields();

    let function = function?;
    let outer_fn_name = get_function_name(function);

    if outer_fn_name != "length" {
        return Ok(None);
    }

    let arguments = arguments?.items();
    let mut arg_is_binary_expr = false;
    let mut operator_text: String = "".to_string();
    let mut lhs: String = "".to_string();
    let mut rhs: String = "".to_string();

    match arguments.into_iter().next() {
        Some(first_arg) => {
            if let Ok(x) = first_arg {
                let RArgumentFields { name_clause: _, value } = x.as_fields();
                let value = value.context("Found named argument without any value")?;
                if let AnyRExpression::RBinaryExpression(y) = value {
                    let RBinaryExpressionFields { left, operator, right } = y.as_fields();

                    let operator = operator?;
                    arg_is_binary_expr = operator.kind() == EQUAL2
                        || operator.kind() == GREATER_THAN
                        || operator.kind() == GREATER_THAN_OR_EQUAL_TO
                        || operator.kind() == LESS_THAN
                        || operator.kind() == LESS_THAN_OR_EQUAL_TO
                        || operator.kind() == NOT_EQUAL;

                    operator_text.push_str(operator.text_trimmed());
                    lhs.push_str(&left?.into_syntax().text_trimmed().to_string());
                    rhs.push_str(&right?.into_syntax().text_trimmed().to_string());
                }
            }
        }
        _ => {
            return Ok(None);
        }
    }

    if arg_is_binary_expr {
        let range = ast.syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(
            LengthTest,
            range,
            Fix {
                content: format!("length({lhs}) {operator_text} {rhs}"),
                start: range.start().into(),
                end: range.end().into(),
            },
        );
        return Ok(Some(diagnostic));
    }

    Ok(None)
}
