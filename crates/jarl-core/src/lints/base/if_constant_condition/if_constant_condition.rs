use crate::diagnostic::*;
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Detects `if` conditions that are always `TRUE` or `FALSE`.
/// This is only triggered for `if` statements without an `else`
/// clause, these are handled by `unreachable_code`
///
/// ## Why is this bad?
///
/// Code with constant conditions will either never run or always run.
/// It clutters the code and makes it more difficult to read.
/// Dead code should be removed and always true code should be unwrapped.
///
/// This rule does not have an automatic fix
///
/// ## Example
///
/// ```r
/// if (TRUE) {
///   print("always true")
/// }
///
/// if (FALSE && ...) {
///   print("always false")
/// }
///
/// if (TRUE || ...) {
///   print("always true")
/// }
/// ```
pub fn if_constant_condition(ast: &RIfStatement) -> anyhow::Result<Option<Diagnostic>> {
    if ast.else_clause().is_some() {
        return Ok(None);
    }

    let condition = ast.condition()?;
    let constant_value = match evaluate_constant_condition(&condition)? {
        Some(value) => value,
        None => return Ok(None),
    };

    let (message, suggestion) = if constant_value {
        (
            "`if` condition is always `TRUE`.".to_string(),
            Some("Remove the `if` condition and keep the body.".to_string()),
        )
    } else {
        (
            "`if` condition is always `FALSE`.".to_string(),
            Some("Remove this `if` statement.".to_string()),
        )
    };

    let range = condition.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new("if_constant_condition".to_string(), message, suggestion),
        range,
        Fix::empty(),
    );

    Ok(Some(diagnostic))
}

fn evaluate_constant_condition(expr: &AnyRExpression) -> anyhow::Result<Option<bool>> {
    match expr {
        // Catch simple if (TRUE) ... or if (FALSE) ...
        AnyRExpression::RTrueExpression(_) => Ok(Some(true)),
        AnyRExpression::RFalseExpression(_) => Ok(Some(false)),

        // Catch if ((TRUE)) or if ((FALSE))
        AnyRExpression::RParenthesizedExpression(children) => {
            let body = children.body()?;
            evaluate_constant_condition(&body)
        }

        // Catch if (!TRUE) or if (!FALSE)
        // NOTE: Maybe this should be its own linter?
        // FALSE should be used instead of !TRUE and so on?
        AnyRExpression::RUnaryExpression(children) => {
            let operator = children.operator()?;
            let operator = operator.text_trimmed();
            if operator != "!" {
                return Ok(None);
            }
            let argument = children.argument()?;
            Ok(evaluate_constant_condition(&argument)?.map(|value| !value))
        }

        // Catch cases with `&&` and `||`
        // NOTE: This won't handle cases of TRUE > FALSE or FALSE > TRUE,
        // but I hope that isn't common...
        AnyRExpression::RBinaryExpression(children) => {
            let RBinaryExpressionFields { left, operator, right } = children.as_fields();
            let operator = operator?;
            let left = left?;
            let right = right?;

            let operator_text = operator.text_trimmed();
            let is_or = matches!(operator_text, "||" | "|");
            let is_and = matches!(operator_text, "&&" | "&");

            if !is_or && !is_and {
                return Ok(None);
            }

            let left_const = evaluate_constant_condition(&left)?;
            let right_const = evaluate_constant_condition(&right)?;

            if is_or {
                // either side of || is true => always true
                if left_const == Some(true) || right_const == Some(true) {
                    return Ok(Some(true));
                }
                // FALSE || x => x
                if left_const == Some(false) {
                    return Ok(right_const);
                }
                Ok(None)
            } else {
                // either side of && is false => always false
                if left_const == Some(false) || right_const == Some(false) {
                    return Ok(Some(false));
                }
                // TRUE && x => x
                if left_const == Some(true) {
                    return Ok(right_const);
                }
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}
