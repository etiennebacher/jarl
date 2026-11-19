use air_r_syntax::*;
use biome_rowan::AstNode;
use oak_core::syntax_ext::{AnyRSelectorExt, RIdentifierExt};

/// Recognize a zero literal or a sequence with both endpoints equal to zero.
pub(super) fn is_zero_index(expression: &AnyRExpression) -> anyhow::Result<bool> {
    match expression {
        AnyRExpression::RParenthesizedExpression(paren) => is_zero_index(&paren.body()?),
        AnyRExpression::RUnaryExpression(unary) => Ok(matches!(
            unary.operator()?.kind(),
            RSyntaxKind::PLUS | RSyntaxKind::MINUS
        ) && is_zero_index(&unary.argument()?)?),
        AnyRExpression::RBinaryExpression(binary)
            if binary.operator()?.kind() == RSyntaxKind::COLON =>
        {
            Ok(numeric_literal(&binary.left()?) == Some(0.0)
                && numeric_literal(&binary.right()?) == Some(0.0))
        }
        expression => Ok(numeric_literal(expression) == Some(0.0)),
    }
}

/// Recognize numeric literals, `:` sequences, and arithmetic built from them
/// without evaluating it. Requiring both arithmetic operands to qualify
/// excludes expressions such as `NULL * -2` and `idx * -2`.
pub(super) fn is_nonempty_numeric_expression(expression: &AnyRExpression) -> anyhow::Result<bool> {
    match expression {
        AnyRExpression::RParenthesizedExpression(paren) => {
            is_nonempty_numeric_expression(&paren.body()?)
        }
        AnyRExpression::AnyRValue(AnyRValue::RIntegerValue(_) | AnyRValue::RDoubleValue(_)) => {
            Ok(true)
        }
        AnyRExpression::RUnaryExpression(unary) => {
            Ok(matches!(
                unary.operator()?.kind(),
                RSyntaxKind::PLUS | RSyntaxKind::MINUS
            ) && is_nonempty_numeric_expression(&unary.argument()?)?)
        }
        AnyRExpression::RBinaryExpression(binary) => {
            let operator = binary.operator()?;
            if operator.kind() == RSyntaxKind::COLON {
                return Ok(true);
            }
            let is_arithmetic = matches!(
                operator.kind(),
                RSyntaxKind::PLUS
                    | RSyntaxKind::MINUS
                    | RSyntaxKind::MULTIPLY
                    | RSyntaxKind::DIVIDE
                    | RSyntaxKind::EXPONENTIATE
                    | RSyntaxKind::EXPONENTIATE2
            ) || (operator.kind() == RSyntaxKind::SPECIAL
                && matches!(operator.text_trimmed(), "%%" | "%/%"));
            Ok(is_arithmetic
                && is_nonempty_numeric_expression(&binary.left()?)?
                && is_nonempty_numeric_expression(&binary.right()?)?)
        }
        _ => Ok(false),
    }
}

/// Recognize a base `c()` call with literals or known numeric expressions.
fn is_literal_c_call(call: &RCall) -> anyhow::Result<bool> {
    if !is_call_to(call, "c", "base")? {
        return Ok(false);
    }
    for arg in call.arguments()?.items() {
        let Some(value) = arg?.value() else {
            return Ok(false);
        };
        if !is_literal(&value)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Recognize a literal value or a numeric expression that cannot be empty.
pub(super) fn is_literal(expression: &AnyRExpression) -> anyhow::Result<bool> {
    match expression {
        AnyRExpression::RParenthesizedExpression(paren) => is_literal(&paren.body()?),
        AnyRExpression::AnyRValue(
            AnyRValue::RIntegerValue(_)
            | AnyRValue::RDoubleValue(_)
            | AnyRValue::RComplexValue(_)
            | AnyRValue::RStringValue(_),
        )
        | AnyRExpression::RNaExpression(_)
        | AnyRExpression::RNullExpression(_)
        | AnyRExpression::RTrueExpression(_)
        | AnyRExpression::RFalseExpression(_)
        | AnyRExpression::RInfExpression(_)
        | AnyRExpression::RNanExpression(_) => Ok(true),
        expression @ AnyRExpression::RBinaryExpression(_) => {
            is_nonempty_numeric_expression(expression)
        }
        AnyRExpression::RCall(call) => is_literal_c_call(call),
        AnyRExpression::RUnaryExpression(unary) => Ok(matches!(
            unary.operator()?.kind(),
            RSyntaxKind::PLUS | RSyntaxKind::MINUS
        ) && is_literal(&unary.argument()?)?),
        _ => Ok(false),
    }
}

/// Read a decimal integer or double as `f64`, including `L` suffixes, unary
/// signs, and parentheses.
pub(super) fn numeric_literal(expression: &AnyRExpression) -> Option<f64> {
    match expression {
        AnyRExpression::RParenthesizedExpression(paren) => numeric_literal(&paren.body().ok()?),
        AnyRExpression::AnyRValue(AnyRValue::RIntegerValue(_) | AnyRValue::RDoubleValue(_)) => {
            expression
                .to_trimmed_string()
                .trim_end_matches('L')
                .parse()
                .ok()
        }
        AnyRExpression::RUnaryExpression(unary) => match unary.operator().ok()?.kind() {
            RSyntaxKind::MINUS => Some(-numeric_literal(&unary.argument().ok()?)?),
            RSyntaxKind::PLUS => numeric_literal(&unary.argument().ok()?),
            _ => None,
        },
        _ => None,
    }
}

/// Strip only the parentheses around the whole expression, so `((-idx))`
/// is checked as `-idx` while `-(idx)` keeps its parenthesized operand.
pub(super) fn unparenthesized(mut expression: AnyRExpression) -> anyhow::Result<AnyRExpression> {
    while let AnyRExpression::RParenthesizedExpression(paren) = expression {
        expression = paren.body()?;
    }
    Ok(expression)
}

/// Read the name and optional package of a plain or namespace-qualified call.
pub(super) fn call_name_and_package(
    call: &RCall,
) -> anyhow::Result<Option<(String, Option<String>)>> {
    Ok(match call.function()? {
        AnyRExpression::RIdentifier(identifier) => Some((identifier.name_text(), None)),
        AnyRExpression::RNamespaceExpression(namespace) => namespace
            .right()?
            .identifier_text()
            .zip(namespace.left()?.identifier_text())
            .map(|(name, package)| (name, Some(package))),
        _ => None,
    })
}

/// Accept an unqualified call or one explicitly qualified with the given package.
pub(super) fn is_call_to(call: &RCall, name: &str, package: &str) -> anyhow::Result<bool> {
    Ok(
        call_name_and_package(call)?.is_some_and(|(actual_name, actual_package)| {
            actual_name == name
                && actual_package
                    .as_deref()
                    .is_none_or(|actual| actual == package)
        }),
    )
}
