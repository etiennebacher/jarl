use crate::utils::{get_arg_by_position, get_function_name, get_function_namespace_prefix};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList, TextSize, WalkEvent};
use oak_core::syntax_ext::RIdentifierExt;

/// A length or scalar check can make a negated variable safe in an enclosing
/// `if` branch. Only trust it while the same variable still holds its value.
pub(super) fn has_nonempty_guard(unary: &RUnaryExpression) -> anyhow::Result<bool> {
    let Some(identifier) = unparenthesized(unary.argument()?)?
        .as_r_identifier()
        .cloned()
    else {
        return Ok(false);
    };
    let name = identifier.name_text();
    let position = unary.range().start();
    let mut end = position;

    for ancestor in unary.syntax().ancestors().skip(1) {
        // A deferred function body may run after the checked variable changes.
        if RFunctionDefinition::can_cast(ancestor.kind()) {
            break;
        }

        // A later iteration can reach this use after a write below it. When
        // the guard is outside a loop, check its whole body for such writes.
        if matches!(
            ancestor.kind(),
            RSyntaxKind::R_FOR_STATEMENT
                | RSyntaxKind::R_WHILE_STATEMENT
                | RSyntaxKind::R_REPEAT_STATEMENT
        ) {
            end = ancestor.text_trimmed_range().end();
        }

        let Some(if_statement) = RIfStatement::cast(ancestor) else {
            continue;
        };
        let condition = if_statement.condition()?;
        let consequence = if_statement.consequence()?;
        let (branch, truth) = if consequence.range().contains(position) {
            (consequence, true)
        } else if let Some(else_clause) = if_statement.else_clause()
            && let alternative = else_clause.alternative()?
            && alternative.range().contains(position)
        {
            (alternative, false)
        } else {
            // The check has not completed when the use is in the condition.
            continue;
        };

        if condition_proves_nonempty(condition.clone(), &name, truth)?
            && !has_write(condition.syntax(), &name, condition.range().end())?
            && !has_write(branch.syntax(), &name, end)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Infer only facts established by reaching this particular branch. Both sides
/// of `&&` hold in its true branch; either side may suffice for `||`, so both
/// must establish the fact there. Negation and `else` reverse these cases.
fn condition_proves_nonempty(
    condition: AnyRExpression,
    name: &str,
    truth: bool,
) -> anyhow::Result<bool> {
    match unparenthesized(condition)? {
        AnyRExpression::RUnaryExpression(unary)
            if unary.operator()?.kind() == RSyntaxKind::BANG =>
        {
            condition_proves_nonempty(unary.argument()?, name, !truth)
        }
        AnyRExpression::RBinaryExpression(binary) => {
            let operator = binary.operator()?.kind();
            let left = binary.left()?;
            let right = binary.right()?;
            if matches!(operator, RSyntaxKind::AND2 | RSyntaxKind::OR2) {
                let left = condition_proves_nonempty(left, name, truth)?;
                let right = condition_proves_nonempty(right, name, truth)?;
                return Ok(if (operator == RSyntaxKind::AND2) == truth {
                    left || right
                } else {
                    left && right
                });
            }

            let left = unparenthesized(left)?;
            let right = unparenthesized(right)?;
            // If the variable itself is compared with a scalar literal, a
            // successful `if` check requires it to be nonempty in either arm.
            if let Some(number) = numeric_literal(&right) {
                comparison_excludes_empty(&left, name, operator, number, truth)
            } else if let Some(number) = numeric_literal(&left) {
                let reversed = match operator {
                    RSyntaxKind::LESS_THAN => RSyntaxKind::GREATER_THAN,
                    RSyntaxKind::LESS_THAN_OR_EQUAL_TO => RSyntaxKind::GREATER_THAN_OR_EQUAL_TO,
                    RSyntaxKind::GREATER_THAN => RSyntaxKind::LESS_THAN,
                    RSyntaxKind::GREATER_THAN_OR_EQUAL_TO => RSyntaxKind::LESS_THAN_OR_EQUAL_TO,
                    _ => operator,
                };
                comparison_excludes_empty(&right, name, reversed, number, truth)
            } else {
                Ok(false)
            }
        }
        _ => Ok(false),
    }
}

fn comparison_excludes_empty(
    expression: &AnyRExpression,
    name: &str,
    operator: RSyntaxKind,
    number: f64,
    truth: bool,
) -> anyhow::Result<bool> {
    // Evaluate the comparison at length zero. A branch excludes empty indices
    // only if this result would send execution to the other branch.
    let empty_result = match operator {
        RSyntaxKind::EQUAL2 => 0.0 == number,
        RSyntaxKind::NOT_EQUAL => 0.0 != number,
        RSyntaxKind::LESS_THAN => 0.0 < number,
        RSyntaxKind::LESS_THAN_OR_EQUAL_TO => 0.0 <= number,
        RSyntaxKind::GREATER_THAN => 0.0 > number,
        RSyntaxKind::GREATER_THAN_OR_EQUAL_TO => 0.0 >= number,
        _ => return Ok(false),
    };
    if let AnyRExpression::RIdentifier(identifier) = expression {
        return Ok(identifier.name_text() == name);
    }
    let AnyRExpression::RCall(call) = expression else {
        return Ok(false);
    };
    let function = call.function()?;
    if get_function_name(function.clone()) != "length"
        || !matches!(
            get_function_namespace_prefix(function).as_deref(),
            None | Some("base::")
        )
        || call.arguments()?.items().len() != 1
    {
        return Ok(false);
    }
    let Some(argument) =
        get_arg_by_position(&call.arguments()?.items(), 1).and_then(|arg| arg.value())
    else {
        return Ok(false);
    };
    Ok(unparenthesized(argument)?
        .as_r_identifier()
        .is_some_and(|identifier| identifier.name_text() == name)
        && empty_result != truth)
}

fn unparenthesized(mut expression: AnyRExpression) -> anyhow::Result<AnyRExpression> {
    while let AnyRExpression::RParenthesizedExpression(paren) = expression {
        expression = paren.body()?;
    }
    Ok(expression)
}

fn numeric_literal(expression: &AnyRExpression) -> Option<f64> {
    match expression {
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

/// Discard a guard if the variable may have been assigned before its use.
/// Writes in nested function definitions do not run when defining the function.
fn has_write(node: &RSyntaxNode, name: &str, end: TextSize) -> anyhow::Result<bool> {
    let mut walk = node.preorder();
    while let Some(event) = walk.next() {
        let WalkEvent::Enter(node) = event else {
            continue;
        };
        if node.text_trimmed_range().start() >= end {
            break;
        }
        if RFunctionDefinition::can_cast(node.kind()) {
            walk.skip_subtree();
            continue;
        }
        if let Some(for_statement) = RForStatement::cast_ref(&node)
            && for_statement
                .variable()?
                .to_trimmed_string()
                .trim_matches('`')
                == name
        {
            return Ok(true);
        }
        let Some(binary) = RBinaryExpression::cast(node) else {
            continue;
        };
        // An assignment containing the use has not written its result yet:
        // `if (length(idx) > 0) idx <- x[-idx]` still has the checked index.
        if binary.range().end() > end {
            continue;
        }
        let target = match binary.operator()?.kind() {
            RSyntaxKind::ASSIGN | RSyntaxKind::SUPER_ASSIGN | RSyntaxKind::EQUAL => {
                binary.left()?
            }
            RSyntaxKind::ASSIGN_RIGHT | RSyntaxKind::SUPER_ASSIGN_RIGHT => binary.right()?,
            _ => continue,
        };
        if modified_name(target)?.as_deref() == Some(name) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Replacing part of an object, or calling a replacement function on it, can
/// also change its length. Follow the modified object rather than its indices.
fn modified_name(expression: AnyRExpression) -> anyhow::Result<Option<String>> {
    let object = match unparenthesized(expression)? {
        AnyRExpression::RIdentifier(identifier) => return Ok(Some(identifier.name_text())),
        AnyRExpression::RSubset(subset) => subset.function()?,
        AnyRExpression::RSubset2(subset) => subset.function()?,
        AnyRExpression::RExtractExpression(extract) => extract.left()?,
        AnyRExpression::RCall(call) => {
            let Some(object) =
                get_arg_by_position(&call.arguments()?.items(), 1).and_then(|arg| arg.value())
            else {
                return Ok(None);
            };
            object
        }
        _ => return Ok(None),
    };
    modified_name(object)
}
