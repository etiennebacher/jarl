use crate::utils::{
    StatementsExt, get_arg_by_name, get_arg_by_position, get_function_name,
    get_function_namespace_prefix,
};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList, TextSize, WalkEvent};
use oak_core::syntax_ext::RIdentifierExt;

/// A check or numeric assignment can establish that a negated variable is
/// nonempty. Only trust that evidence while the variable holds its value.
pub(super) fn has_nonempty_guard(unary: &RUnaryExpression) -> anyhow::Result<bool> {
    let AnyRExpression::RIdentifier(identifier) = unparenthesized(unary.argument()?)? else {
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

        // Collect protection from enclosing conditions and earlier statements.
        // Each checker handles its own control flow and uses the same condition
        // and assignment analysis below.
        let guarded = if let Some(statements) = RStatementList::cast_ref(&ancestor) {
            has_preceding_guard(&statements, &name, position, end)?
        } else {
            match AnyRExpression::cast_ref(&ancestor) {
                Some(AnyRExpression::RIfStatement(statement)) => {
                    has_if_guard(&statement, &name, position, end)?
                }
                Some(AnyRExpression::RWhileStatement(statement)) => {
                    has_while_guard(&statement, &name, position, end)?
                }
                _ => false,
            }
        };
        if guarded {
            return Ok(true);
        }

        // `while` rechecks its own condition on each iteration. Only guards
        // outside a loop must account for writes after the use in its body.
        if matches!(
            ancestor.kind(),
            RSyntaxKind::R_FOR_STATEMENT
                | RSyntaxKind::R_WHILE_STATEMENT
                | RSyntaxKind::R_REPEAT_STATEMENT
        ) {
            end = ancestor.text_trimmed_range().end();
        }
    }
    Ok(false)
}

/// Use the condition's truth value for the branch containing this index.
/// An index in the condition itself has not yet passed that check.
fn has_if_guard(
    statement: &RIfStatement,
    name: &str,
    position: TextSize,
    end: TextSize,
) -> anyhow::Result<bool> {
    let consequence = statement.consequence()?;
    if consequence.range().contains(position) {
        return branch_is_guarded(statement.condition()?, consequence, name, true, end);
    }
    if let Some(else_clause) = statement.else_clause()
        && let alternative = else_clause.alternative()?
        && alternative.range().contains(position)
    {
        return branch_is_guarded(statement.condition()?, alternative, name, false, end);
    }
    Ok(false)
}

/// The while condition is true when entering its body on each iteration.
fn has_while_guard(
    statement: &RWhileStatement,
    name: &str,
    position: TextSize,
    end: TextSize,
) -> anyhow::Result<bool> {
    let body = statement.body()?;
    Ok(body.range().contains(position)
        && branch_is_guarded(statement.condition()?, body, name, true, end)?)
}

/// A condition protects the selected branch only while its checked value is
/// unchanged, including assignments performed while evaluating the condition.
fn branch_is_guarded(
    condition: AnyRExpression,
    branch: AnyRExpression,
    name: &str,
    truth: bool,
    end: TextSize,
) -> anyhow::Result<bool> {
    Ok(
        condition_proves_nonempty(condition.clone(), name, truth, false)?
            && !has_write(condition.syntax(), name, condition.range().end())?
            && !has_write(branch.syntax(), name, end)?,
    )
}

/// Look backward for a guard, stopping at a write that would invalidate it.
/// When the use is in a loop, `end` also includes writes that affect subsequent
/// iterations. A check after the use cannot protect it.
fn has_preceding_guard(
    statements: &RStatementList,
    name: &str,
    position: TextSize,
    end: TextSize,
) -> anyhow::Result<bool> {
    for expression in statements.expressions().into_iter().rev() {
        if expression.range().start() >= end {
            continue;
        }
        // Check the guard first: writes on its exiting path do not matter.
        if expression.range().end() <= position {
            let guarded = match unparenthesized(expression.clone())? {
                AnyRExpression::RIfStatement(statement) => is_early_exit_guard(&statement, name)?,
                AnyRExpression::RCall(call) => is_stopifnot_guard(&call, name)?,
                AnyRExpression::RBinaryExpression(binary) => is_numeric_assignment(&binary, name)?,
                _ => false,
            };
            if guarded {
                return Ok(true);
            }
        }
        if has_write(expression.syntax(), name, end)? {
            return Ok(false);
        }
    }
    Ok(false)
}

/// A direct local assignment establishes nonemptiness on every path reaching
/// the next statement. Conditional writes and replacement assignments cannot
/// establish this fact; the caller still treats them as invalidating writes.
fn is_numeric_assignment(binary: &RBinaryExpression, name: &str) -> anyhow::Result<bool> {
    let (target, value) = match binary.operator()?.kind() {
        RSyntaxKind::ASSIGN | RSyntaxKind::EQUAL => (binary.left()?, binary.right()?),
        RSyntaxKind::ASSIGN_RIGHT => (binary.right()?, binary.left()?),
        _ => return Ok(false),
    };
    let AnyRExpression::RIdentifier(identifier) = target else {
        return Ok(false);
    };
    Ok(identifier.name_text() == name && is_numeric_literal(value)?)
}

/// Only scalar numeric literals establish this fact, without evaluating calls
/// or arithmetic. Inspect the syntax so decimal and hexadecimal forms agree.
fn is_numeric_literal(expression: AnyRExpression) -> anyhow::Result<bool> {
    match unparenthesized(expression)? {
        AnyRExpression::AnyRValue(AnyRValue::RIntegerValue(_) | AnyRValue::RDoubleValue(_)) => {
            Ok(true)
        }
        AnyRExpression::RUnaryExpression(unary) => Ok(matches!(
            unary.operator()?.kind(),
            RSyntaxKind::PLUS | RSyntaxKind::MINUS
        ) && is_numeric_literal(unary.argument()?)?),
        _ => Ok(false),
    }
}

/// The surviving branch must establish nonemptiness and preserve the variable.
/// Writes in the exiting branch cannot affect code after the `if`.
fn is_early_exit_guard(statement: &RIfStatement, name: &str) -> anyhow::Result<bool> {
    let condition = statement.condition()?;
    if has_write(condition.syntax(), name, condition.range().end())? {
        return Ok(false);
    }
    let consequence = statement.consequence()?;
    let alternative = statement
        .else_clause()
        .map(|clause| clause.alternative())
        .transpose()?;

    if always_exits(consequence.clone())?
        && condition_proves_nonempty(condition.clone(), name, false, false)?
    {
        return match alternative {
            Some(branch) => Ok(!has_write(branch.syntax(), name, branch.range().end())?),
            None => Ok(true),
        };
    }
    if let Some(alternative) = alternative
        && always_exits(alternative)?
        && condition_proves_nonempty(condition, name, true, false)?
    {
        return Ok(!has_write(
            consequence.syntax(),
            name,
            consequence.range().end(),
        )?);
    }
    Ok(false)
}

/// Require an unconditional exit from this branch. Inspect only statements
/// that execute here: a nested function, loop, or arbitrary call may contain
/// an exit without preventing execution of the following statement.
fn always_exits(expression: AnyRExpression) -> anyhow::Result<bool> {
    match unparenthesized(expression)? {
        AnyRExpression::RBreakExpression(_) | AnyRExpression::RNextExpression(_) => Ok(true),
        AnyRExpression::RCall(call) => {
            Ok(is_base_call(&call, "return")? || is_base_call(&call, "stop")?)
        }
        AnyRExpression::RBracedStatements(braced) => {
            for expression in braced.expressions() {
                if always_exits(expression)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        AnyRExpression::RIfStatement(statement) => {
            let Some(alternative) = statement.else_clause() else {
                return Ok(false);
            };
            Ok(
                always_exits(statement.consequence()?)?
                    && always_exits(alternative.alternative()?)?,
            )
        }
        _ => Ok(false),
    }
}

/// Infer only facts established by reaching this particular branch. Both sides
/// of `&&` hold in its true branch; either side may suffice for `||`, so both
/// must establish the fact there. Negation and `else` reverse these cases.
/// `allow_empty` accounts for `stopifnot()`, which accepts `logical(0)`.
fn condition_proves_nonempty(
    condition: AnyRExpression,
    name: &str,
    truth: bool,
    allow_empty: bool,
) -> anyhow::Result<bool> {
    match unparenthesized(condition)? {
        // A length of zero cannot enter the true branch of `if (length(idx))`.
        AnyRExpression::RCall(call) => {
            Ok(truth && (is_index_length(&call, name)? || is_any_guard(&call, name)?))
        }
        AnyRExpression::RUnaryExpression(unary)
            if unary.operator()?.kind() == RSyntaxKind::BANG =>
        {
            condition_proves_nonempty(unary.argument()?, name, !truth, allow_empty)
        }
        AnyRExpression::RBinaryExpression(binary) => {
            let operator = binary.operator()?.kind();
            if matches!(operator, RSyntaxKind::AND2 | RSyntaxKind::OR2) {
                // Short-circuit operators always return a scalar, even inside
                // `stopifnot()`, so their operands cannot pass as empty results.
                let left = condition_proves_nonempty(binary.left()?, name, truth, false)?;
                let right = condition_proves_nonempty(binary.right()?, name, truth, false)?;
                return Ok(if (operator == RSyntaxKind::AND2) == truth {
                    left || right
                } else {
                    left && right
                });
            }

            comparison_proves_nonempty(&binary, name, truth, allow_empty)
        }
        _ => Ok(false),
    }
}

/// Compare the variable or its length with a numeric literal. Normalize the
/// literal to the right so reversed comparisons use the same empty-value test.
fn comparison_proves_nonempty(
    binary: &RBinaryExpression,
    name: &str,
    truth: bool,
    allow_empty: bool,
) -> anyhow::Result<bool> {
    let left = unparenthesized(binary.left()?)?;
    let right = unparenthesized(binary.right()?)?;
    let mut operator = binary.operator()?.kind();
    let (expression, number) = if let Some(number) = numeric_literal(&right) {
        (left, number)
    } else if let Some(number) = numeric_literal(&left) {
        operator = match operator {
            RSyntaxKind::LESS_THAN => RSyntaxKind::GREATER_THAN,
            RSyntaxKind::LESS_THAN_OR_EQUAL_TO => RSyntaxKind::GREATER_THAN_OR_EQUAL_TO,
            RSyntaxKind::GREATER_THAN => RSyntaxKind::LESS_THAN,
            RSyntaxKind::GREATER_THAN_OR_EQUAL_TO => RSyntaxKind::LESS_THAN_OR_EQUAL_TO,
            _ => operator,
        };
        (right, number)
    } else {
        return Ok(false);
    };
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
    // Direct vector comparisons can pass stopifnot() with an empty result.
    // `if` and a true `any()` require at least one result instead.
    if let AnyRExpression::RIdentifier(identifier) = expression {
        return Ok(!allow_empty && identifier.name_text() == name);
    }
    let AnyRExpression::RCall(call) = expression else {
        return Ok(false);
    };
    Ok(empty_result != truth && is_index_length(&call, name)?)
}

/// A true `any()` needs at least one true element. Only accept one data
/// argument: another argument could make it true while this index is empty.
fn is_any_guard(call: &RCall, name: &str) -> anyhow::Result<bool> {
    if !is_base_call(call, "any")? {
        return Ok(false);
    }
    let arguments = call.arguments()?.items();
    let na_rm = get_arg_by_name(&arguments, "na.rm");
    let mut condition = None;
    for argument in arguments {
        let argument = argument?;
        if na_rm
            .as_ref()
            .is_some_and(|na_rm| na_rm.syntax() == argument.syntax())
        {
            continue;
        }
        if condition.is_some() {
            return Ok(false);
        }
        let Some(value) = argument.value() else {
            return Ok(false);
        };
        condition = Some(value);
    }
    match condition {
        Some(condition) => condition_proves_nonempty(condition, name, true, false),
        None => Ok(false),
    }
}

/// A successful assertion protects following statements only if an empty
/// index would fail it. `stopifnot(idx > 0)` accepts empty vectors, whereas
/// `stopifnot(length(idx) > 0)` and `stopifnot(any(idx > 0))` reject them.
fn is_stopifnot_guard(call: &RCall, name: &str) -> anyhow::Result<bool> {
    if !is_base_call(call, "stopifnot")? || has_write(call.syntax(), name, call.range().end())? {
        return Ok(false);
    }
    let arguments = call.arguments()?.items();
    // These control how assertions are evaluated; they are not ordinary
    // assertion arguments. Do not infer facts for these call forms.
    if ["exprs", "exprObject", "local"]
        .iter()
        .any(|name| get_arg_by_name(&arguments, name).is_some())
    {
        return Ok(false);
    }
    let mut guarded = false;
    for argument in arguments {
        let argument = argument?;
        let Some(condition) = argument.value() else {
            return Ok(false);
        };
        guarded |= condition_proves_nonempty(condition, name, true, true)?;
    }
    Ok(guarded)
}

/// Recognize the length of this variable, including `base::length(x = idx)`.
fn is_index_length(call: &RCall, name: &str) -> anyhow::Result<bool> {
    if !is_base_call(call, "length")? || call.arguments()?.items().len() != 1 {
        return Ok(false);
    }
    let Some(argument) =
        get_arg_by_position(&call.arguments()?.items(), 1).and_then(|arg| arg.value())
    else {
        return Ok(false);
    };
    Ok(unparenthesized(argument)?
        .as_r_identifier()
        .is_some_and(|identifier| identifier.name_text() == name))
}

/// Apply base semantics only to unqualified calls and known base functions.
fn is_base_call(call: &RCall, name: &str) -> anyhow::Result<bool> {
    let function = call.function()?;
    Ok(get_function_name(function.clone()) == name
        && matches!(
            get_function_namespace_prefix(function).as_deref(),
            None | Some("base::")
        ))
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
