use crate::diagnostic::*;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct IfAlwaysTrue;

/// ## What it does
///
/// Detects `if` conditions that always evaluate to `TRUE`. This is only triggered
/// for `if` statements without an `else` clause, these are handled by
/// `unreachable_code`.
///
/// ## Why is this bad?
///
/// Code in an `if` statement whose condition always evaluates to `TRUE` will
/// always run. It clutters the code and makes it more difficult to read. In
/// these cases, the `if` condition should be removed.
///
/// This rule does not have an automatic fix.
///
/// ## Example
///
/// ```r
/// if (TRUE) {
///   print("always true")
/// }
///
/// if (TRUE || ...) {
///   print("always true")
/// }
///
/// if (!FALSE) {
///   print("always true")
/// }
/// ```
///
/// Use instead:
///
/// ```r
/// print("always true")
/// ```
impl Violation for IfAlwaysTrue {
    fn name(&self) -> String {
        "if_always_true".to_string()
    }
    fn body(&self) -> String {
        "`if` condition always evaluates to `TRUE`.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Modify the `if` condition, or keep only the body.".to_string())
    }
}

pub fn if_always_true(ast: &RIfStatement) -> anyhow::Result<Option<Diagnostic>> {
    // This is already handled by `unreachable_code`
    if ast.else_clause().is_some() {
        return Ok(None);
    }

    let condition = ast.condition()?;

    if evaluate_constant_condition(&condition)? != Some(true) {
        return Ok(None);
    }

    let range = condition.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(IfAlwaysTrue, range, Fix::empty());

    Ok(Some(diagnostic))
}

fn evaluate_constant_condition(expr: &AnyRExpression) -> anyhow::Result<Option<bool>> {
    match expr {
        // Catch simple if (TRUE) ... or if (FALSE) ...
        AnyRExpression::RTrueExpression(_) => Ok(Some(true)),
        AnyRExpression::RFalseExpression(_) => Ok(Some(false)),

        // Catch Inf and -Inf which are always TRUE
        AnyRExpression::RInfExpression(_) => Ok(Some(true)),

        // 0 is always FALSE, any other number in R is TRUE
        AnyRExpression::AnyRValue(value) => {
            if let Some(int) = value.as_r_integer_value() {
                let token = int.value_token()?;
                let text = token.text_trimmed();
                let normalized = text.strip_suffix('L').unwrap_or(text);
                let value: i64 = normalized.parse()?;
                return Ok(Some(value != 0));
            }
            if let Some(double) = value.as_r_double_value() {
                let token = double.value_token()?;
                let text = token.text_trimmed();
                let value: f64 = if text.starts_with('.') {
                    format!("0{text}").parse()?
                } else {
                    text.parse()?
                };
                if value.is_nan() {
                    return Ok(None);
                }
                return Ok(Some(value != 0.0));
            }
            Ok(None)
        }

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
            let argument = children.argument()?;
            if operator == "!" {
                Ok(evaluate_constant_condition(&argument)?.map(|value| !value))
            } else if operator == "-" || operator == "+" {
                evaluate_constant_condition(&argument)
            } else {
                Ok(None)
            }
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
                // either side of || is TRUE => always TRUE
                if left_const == Some(true) || right_const == Some(true) {
                    return Ok(Some(true));
                }
                // FALSE || x => x
                if left_const == Some(false) {
                    return Ok(right_const);
                }
                Ok(None)
            } else {
                // either side of && is FALSE => always FALSE
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
