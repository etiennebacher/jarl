use crate::utils::{StatementsExt, get_function_name};
use air_r_syntax::*;
use biome_rowan::AstNode;
use oak_core::syntax_ext::RIdentifierExt;

use super::negative_index::unparenthesized;
use super::nonempty_guard::{has_write, index_ancestors, numeric_literal};

pub(super) enum LiteralIndex {
    Literal,
    Zero,
    NonLiteral,
}

/// Classify the operand of the negative index. Direct literals stay exempt;
/// zero gets a separate result when it comes from an assignment.
pub(super) fn is_literal_index(unary: &RUnaryExpression) -> anyhow::Result<LiteralIndex> {
    let operand = unparenthesized(unary.argument()?)?;
    let AnyRExpression::RIdentifier(identifier) = operand else {
        return Ok(if is_literal(operand)? {
            LiteralIndex::Literal
        } else {
            LiteralIndex::NonLiteral
        });
    };

    let name = identifier.name_text();
    let position = unary.range().start();
    for (ancestor, end) in index_ancestors(unary) {
        let Some(statements) = RStatementList::cast(ancestor) else {
            continue;
        };
        for statement in statements.expressions().into_iter().rev() {
            if statement.range().start() >= end {
                continue;
            }

            if statement.range().end() <= position
                && let AnyRExpression::RBinaryExpression(binary) =
                    unparenthesized(statement.clone())?
                && let Some(value) = assigned_value(&binary, &name)?
            {
                return classify_assigned_value(value);
            }

            if has_write(statement.syntax(), &name, end)? {
                return Ok(LiteralIndex::NonLiteral);
            }
        }
    }
    Ok(LiteralIndex::NonLiteral)
}

/// Recognize a direct local assignment to the index variable. Replacement and
/// conditional assignments can invalidate earlier values without proving a new one.
fn assigned_value(
    binary: &RBinaryExpression,
    name: &str,
) -> anyhow::Result<Option<AnyRExpression>> {
    let (target, value) = match binary.operator()?.kind() {
        RSyntaxKind::ASSIGN | RSyntaxKind::EQUAL => (binary.left()?, binary.right()?),
        RSyntaxKind::ASSIGN_RIGHT => (binary.right()?, binary.left()?),
        _ => return Ok(None),
    };
    Ok(target
        .as_r_identifier()
        .is_some_and(|identifier| identifier.name_text() == name)
        .then_some(value))
}

fn classify_assigned_value(value: AnyRExpression) -> anyhow::Result<LiteralIndex> {
    if !is_constant_numeric_expression(value.clone())? {
        return Ok(LiteralIndex::NonLiteral);
    }
    Ok(if is_zero_literal(value)? {
        LiteralIndex::Zero
    } else {
        LiteralIndex::Literal
    })
}

/// Zero has length one, but `x[-0]` selects nothing.
fn is_zero_literal(expression: AnyRExpression) -> anyhow::Result<bool> {
    match unparenthesized(expression)? {
        AnyRExpression::RUnaryExpression(unary) => is_zero_literal(unary.argument()?),
        expression => Ok(numeric_literal(&expression) == Some(0.0)),
    }
}

/// Recognize arithmetic built only from numeric scalars, without evaluating it.
/// Requiring both operands to qualify excludes empty or runtime values, such as
/// `NULL * -2` and `idx * -2`.
fn is_constant_numeric_expression(expression: AnyRExpression) -> anyhow::Result<bool> {
    match unparenthesized(expression)? {
        AnyRExpression::AnyRValue(AnyRValue::RIntegerValue(_) | AnyRValue::RDoubleValue(_)) => {
            Ok(true)
        }
        AnyRExpression::RUnaryExpression(unary) => {
            Ok(matches!(
                unary.operator()?.kind(),
                RSyntaxKind::PLUS | RSyntaxKind::MINUS
            ) && is_constant_numeric_expression(unary.argument()?)?)
        }
        AnyRExpression::RBinaryExpression(binary) => {
            let operator = binary.operator()?;
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
                && is_constant_numeric_expression(binary.left()?)?
                && is_constant_numeric_expression(binary.right()?)?)
        }
        _ => Ok(false),
    }
}

/// A `c()` made only of literals has no runtime-dependent values to warn about.
fn is_literal_c_call(call: &RCall) -> anyhow::Result<bool> {
    if get_function_name(call.function()?) != "c" {
        return Ok(false);
    }
    for arg in call.arguments()?.items() {
        let Some(value) = arg?.value() else {
            return Ok(false);
        };
        if !is_literal(value)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Recognize a literal value, including `NA`, `NULL`, and `Inf`.
fn is_literal(expression: AnyRExpression) -> anyhow::Result<bool> {
    match unparenthesized(expression)? {
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
            is_constant_numeric_expression(expression)
        }
        AnyRExpression::RCall(call) => is_literal_c_call(&call),
        AnyRExpression::RUnaryExpression(unary) => Ok(matches!(
            unary.operator()?.kind(),
            RSyntaxKind::PLUS | RSyntaxKind::MINUS
        ) && is_literal(unary.argument()?)?),
        _ => Ok(false),
    }
}
