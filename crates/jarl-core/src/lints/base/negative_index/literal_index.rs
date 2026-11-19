use crate::utils::{argument_name, get_arg_by_position};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};
use oak_core::syntax_ext::{AnyRSelectorExt, RIdentifierExt};

/// Recognize a zero literal, an all-zero `c()` call, or a `0:0` sequence.
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
        AnyRExpression::RCall(call) => Ok(matches!(classify_c_call(call)?, CIndex::Zero)),
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

enum CIndex {
    Positive,
    Zero,
    Empty,
    Literal,
    Unknown,
}

/// Under the default `c()` semantics, one positive subscript is enough to
/// avoid an empty or all-zero index. Without one, retain the literal check and
/// distinguish an all-zero result from a call with only `NULL` arguments.
fn classify_c_call(call: &RCall) -> anyhow::Result<CIndex> {
    if !is_call_to(call, "c", "base")? {
        return Ok(CIndex::Unknown);
    }
    let mut result = CIndex::Empty;
    for arg in call.arguments()?.items() {
        let arg = arg?;
        if matches!(
            argument_name(&arg).as_deref(),
            Some("recursive" | "use.names")
        ) {
            continue;
        }
        let Some(value) = arg.value() else {
            return Ok(CIndex::Unknown);
        };
        result = match (result, classify_c_argument(unparenthesized(value)?)?) {
            (_, CIndex::Positive) | (CIndex::Positive, _) => return Ok(CIndex::Positive),
            (CIndex::Unknown, _) | (_, CIndex::Unknown) => CIndex::Unknown,
            (CIndex::Literal, _) | (_, CIndex::Literal) => CIndex::Literal,
            (CIndex::Zero, _) | (_, CIndex::Zero) => CIndex::Zero,
            (CIndex::Empty, CIndex::Empty) => CIndex::Empty,
        };
    }
    Ok(result)
}

fn classify_c_argument(value: AnyRExpression) -> anyhow::Result<CIndex> {
    if let Some(number) = numeric_literal(&value) {
        return Ok(if is_positive_subscript(number) {
            CIndex::Positive
        } else if number == 0.0 {
            CIndex::Zero
        } else {
            CIndex::Literal
        });
    }
    match &value {
        AnyRExpression::RNullExpression(_) => Ok(CIndex::Empty),
        AnyRExpression::RFalseExpression(_) => Ok(CIndex::Zero),
        AnyRExpression::RCall(call) => classify_c_call(call),
        AnyRExpression::RBinaryExpression(binary)
            if binary.operator()?.kind() == RSyntaxKind::COLON
                && sequence_has_positive_subscript(binary)? =>
        {
            Ok(CIndex::Positive)
        }
        _ if is_zero_index(&value)? => Ok(CIndex::Zero),
        _ if is_literal(&value)? => Ok(CIndex::Literal),
        _ => Ok(CIndex::Unknown),
    }
}

fn is_positive_subscript(number: f64) -> bool {
    number.is_finite() && number >= 1.0
}

/// Recognize sequences that, if they yield any values, yield positive integers.
pub(super) fn is_positive_index_sequence(expression: AnyRExpression) -> anyhow::Result<bool> {
    match unparenthesized(expression)? {
        AnyRExpression::RCall(call) => Ok(call_name_and_package(&call)?.is_some_and(
            |(name, package)| {
                matches!(package.as_deref(), None | Some("base"))
                    && matches!(name.as_str(), "seq_along" | "seq_len" | "which")
            },
        )),
        AnyRExpression::RBinaryExpression(binary)
            if binary.operator()?.kind() == RSyntaxKind::COLON =>
        {
            let positive_integer = |number| is_positive_subscript(number) && number.fract() == 0.0;
            Ok(
                numeric_literal(&binary.left()?).is_some_and(positive_integer)
                    && numeric_literal(&binary.right()?).is_some_and(positive_integer),
            )
        }
        _ => Ok(false),
    }
}

/// A `:` sequence always includes its start, but steps by one and can miss
/// its end. For instance, `0.5:1` contains only `0.5`, which truncates to zero.
fn sequence_has_positive_subscript(binary: &RBinaryExpression) -> anyhow::Result<bool> {
    let start = numeric_literal(&binary.left()?);
    if start.is_some_and(is_positive_subscript) {
        return Ok(true);
    }
    let Some(end) = numeric_literal(&binary.right()?).filter(|end| is_positive_subscript(*end))
    else {
        return Ok(false);
    };
    Ok(end >= 2.0
        || start.is_some_and(|start| start.is_finite() && (1.0 - start).ceil() <= end - start))
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
        AnyRExpression::RCall(call) => Ok(!matches!(classify_c_call(call)?, CIndex::Unknown)),
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

/// Match one argument referring to a variable, including named arguments such
/// as `length(x = idx)`.
pub(super) fn has_single_variable_argument(call: &RCall, name: &str) -> anyhow::Result<bool> {
    let arguments = call.arguments()?.items();
    if arguments.len() != 1 {
        return Ok(false);
    }
    let Some(argument) = get_arg_by_position(&arguments, 1).and_then(|arg| arg.value()) else {
        return Ok(false);
    };
    Ok(unparenthesized(argument)?
        .as_r_identifier()
        .is_some_and(|identifier| identifier.name_text() == name))
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
