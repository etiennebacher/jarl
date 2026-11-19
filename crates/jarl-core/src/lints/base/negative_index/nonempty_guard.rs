use crate::utils::{StatementsExt, get_arg_by_name, get_arg_by_position};
use air_r_syntax::*;
use biome_rowan::{AstNode, TextSize, WalkEvent};
use oak_core::syntax_ext::RIdentifierExt;

use super::literal_index::{
    call_name_and_package, has_single_variable_argument, is_call_to,
    is_nonempty_numeric_expression, is_positive_index_sequence, is_zero_index, numeric_literal,
    unparenthesized,
};

#[derive(PartialEq, Eq)]
pub(super) enum IdentifierIndex {
    Safe,
    DefiniteZero,
    Unguarded,
}

pub(super) fn check_identifier_index(
    unary: &RUnaryExpression,
    identifier: &RIdentifier,
) -> anyhow::Result<IdentifierIndex> {
    let name = identifier.name_text();
    let position = unary.range().start();

    // An enclosing length check also accepts a scalar zero. Look for an
    // assignment first so `idx <- 0L; if (length(idx)) x[-idx]` still warns.
    if let Some(value) = preceding_assignment(unary, &name, position)? {
        let value = unparenthesized(value)?;
        if matches!(value, AnyRExpression::RFalseExpression(_))
            || is_nonempty_numeric_expression(&value)?
        {
            return Ok(if is_zero_index(&value)? {
                IdentifierIndex::DefiniteZero
            } else {
                IdentifierIndex::Safe
            });
        }
    }

    guarded_index(unary, &name, position)
}

/// Find the preceding direct assignment, stopping at any intervening write
/// whose value cannot be established.
fn preceding_assignment(
    unary: &RUnaryExpression,
    name: &str,
    position: TextSize,
) -> anyhow::Result<Option<AnyRExpression>> {
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
                && let Some(value) = assigned_value(&binary, name)?
            {
                return Ok(Some(value));
            }
            if has_write(statement.syntax(), name, end)? {
                return Ok(None);
            }
        }
    }
    Ok(None)
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

/// Look for a guard or an enclosing condition that proves the index is zero.
fn guarded_index(
    unary: &RUnaryExpression,
    name: &str,
    position: TextSize,
) -> anyhow::Result<IdentifierIndex> {
    let mut result = IdentifierIndex::Unguarded;
    for (ancestor, end) in index_ancestors(unary) {
        if let Some(statements) = RStatementList::cast_ref(&ancestor) {
            if result != IdentifierIndex::Safe
                && has_preceding_guard(&statements, name, position, end)?
            {
                result = IdentifierIndex::Safe;
            }
            continue;
        }
        let branch = match AnyRExpression::cast_ref(&ancestor) {
            Some(AnyRExpression::RIfStatement(statement)) => {
                if_index_status(&statement, name, position, end)?
            }
            Some(AnyRExpression::RWhileStatement(statement)) => {
                while_index_status(&statement, name, position, end)?
            }
            Some(AnyRExpression::RForStatement(statement))
                if for_index_is_positive(&statement, name, position, end)? =>
            {
                IdentifierIndex::Safe
            }
            _ => IdentifierIndex::Unguarded,
        };
        match branch {
            IdentifierIndex::DefiniteZero => return Ok(IdentifierIndex::DefiniteZero),
            IdentifierIndex::Safe => result = IdentifierIndex::Safe,
            IdentifierIndex::Unguarded => {}
        }
    }
    Ok(result)
}

/// Walk outward within the current function. For evidence outside a loop,
/// include writes after the index use that can affect the next iteration.
fn index_ancestors(unary: &RUnaryExpression) -> impl Iterator<Item = (RSyntaxNode, TextSize)> {
    unary
        .syntax()
        .ancestors()
        .skip(1)
        .take_while(|ancestor| !RFunctionDefinition::can_cast(ancestor.kind()))
        .scan(unary.range().start(), |end, ancestor| {
            let current_end = *end;
            if matches!(
                ancestor.kind(),
                RSyntaxKind::R_FOR_STATEMENT
                    | RSyntaxKind::R_WHILE_STATEMENT
                    | RSyntaxKind::R_REPEAT_STATEMENT
            ) {
                *end = ancestor.text_trimmed_range().end();
            }
            Some((ancestor, current_end))
        })
}

/// Use the condition's truth value for the branch containing this index.
/// An index in the condition itself has not yet passed that check.
fn if_index_status(
    statement: &RIfStatement,
    name: &str,
    position: TextSize,
    end: TextSize,
) -> anyhow::Result<IdentifierIndex> {
    let consequence = statement.consequence()?;
    if consequence.range().contains(position) {
        return branch_index_status(statement.condition()?, consequence, name, true, end);
    }
    if let Some(else_clause) = statement.else_clause() {
        let alternative = else_clause.alternative()?;
        if alternative.range().contains(position) {
            return branch_index_status(statement.condition()?, alternative, name, false, end);
        }
    }
    Ok(IdentifierIndex::Unguarded)
}

/// The while condition is true when entering its body on each iteration.
fn while_index_status(
    statement: &RWhileStatement,
    name: &str,
    position: TextSize,
    end: TextSize,
) -> anyhow::Result<IdentifierIndex> {
    let body = statement.body()?;
    if !body.range().contains(position) {
        return Ok(IdentifierIndex::Unguarded);
    }
    branch_index_status(statement.condition()?, body, name, true, end)
}

/// A `for` loop rebinds its own index on each iteration. Scan only the body:
/// a write after the use in this iteration does not affect the next binding.
fn for_index_is_positive(
    statement: &RForStatement,
    name: &str,
    position: TextSize,
    end: TextSize,
) -> anyhow::Result<bool> {
    let AnyRForVariable::RIdentifier(variable) = statement.variable()? else {
        return Ok(false);
    };
    if variable.name_text() != name {
        return Ok(false);
    }
    let body = statement.body()?;
    Ok(body.range().contains(position)
        && is_positive_index_sequence(statement.sequence()?)?
        && !has_write(body.syntax(), name, end)?)
}

/// Evidence from a condition applies only while the checked value is unchanged.
fn branch_index_status(
    condition: AnyRExpression,
    branch: AnyRExpression,
    name: &str,
    truth: bool,
    end: TextSize,
) -> anyhow::Result<IdentifierIndex> {
    let status = condition_index_status(condition.clone(), name, truth, false)?;
    if matches!(status, IdentifierIndex::Unguarded)
        || has_write(condition.syntax(), name, condition.range().end())?
        || has_write(branch.syntax(), name, end)?
    {
        return Ok(IdentifierIndex::Unguarded);
    }
    Ok(status)
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

/// An early exit protects later uses when the condition on the surviving path
/// proves nonemptiness and that path preserves the variable. Writes confined
/// to the exiting branch cannot affect code after the `if`.
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

    // Select the path that can reach the next statement. For example,
    // `if (!length(idx)) stop("empty")` survives when the condition is false.
    let surviving_branch = if always_exits(consequence.clone())?
        && condition_proves_nonempty(condition.clone(), name, false, false)?
    {
        alternative
    } else if let Some(alternative) = alternative
        && always_exits(alternative)?
        && condition_proves_nonempty(condition, name, true, false)?
    {
        Some(consequence)
    } else {
        return Ok(false);
    };

    // A surviving branch can invalidate the check, as in
    // `if (!length(idx)) stop("empty") else idx <- f()`.
    // Without an else branch, the false path performs no further writes.
    match surviving_branch {
        Some(branch) => Ok(!has_write(branch.syntax(), name, branch.range().end())?),
        None => Ok(true),
    }
}

/// Require an unconditional exit from this branch. Inspect only statements
/// that execute here: a nested function, loop, or arbitrary call may contain
/// an exit without preventing execution of the following statement.
fn always_exits(expression: AnyRExpression) -> anyhow::Result<bool> {
    match unparenthesized(expression)? {
        AnyRExpression::RBreakExpression(_) | AnyRExpression::RNextExpression(_) => Ok(true),
        AnyRExpression::RCall(call) => is_exit_call(&call),
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

fn is_exit_call(call: &RCall) -> anyhow::Result<bool> {
    let Some((name, package)) = call_name_and_package(call)? else {
        return Ok(false);
    };
    let expected_package = match name.as_str() {
        "return" | "stop" | ".Defunct" | "q" | "quit" => "base",
        "abort" => "rlang",
        "cli_abort" => "cli",
        _ => return Ok(false),
    };
    Ok(package
        .as_deref()
        .is_none_or(|actual| actual == expected_package))
}

/// Determine whether observing `truth` for this condition rules out an empty
/// index. Negation reverses the required truth value. `allow_empty` distinguishes
/// assertions such as `stopifnot(idx > 0)`, which also accept `logical(0)`.
fn condition_proves_nonempty(
    condition: AnyRExpression,
    name: &str,
    truth: bool,
    allow_empty: bool,
) -> anyhow::Result<bool> {
    Ok(matches!(
        condition_index_status(condition, name, truth, allow_empty)?,
        IdentifierIndex::Safe
    ))
}

fn condition_index_status(
    condition: AnyRExpression,
    name: &str,
    truth: bool,
    allow_empty: bool,
) -> anyhow::Result<IdentifierIndex> {
    let guarded = match unparenthesized(condition)? {
        // A length of zero cannot enter the true branch of `if (length(idx))`.
        AnyRExpression::RCall(call) => {
            truth && (is_index_length(&call, name)? || is_any_guard(&call, name)?)
        }
        AnyRExpression::RUnaryExpression(unary)
            if unary.operator()?.kind() == RSyntaxKind::BANG =>
        {
            return condition_index_status(unary.argument()?, name, !truth, allow_empty);
        }
        AnyRExpression::RBinaryExpression(binary) => {
            let operator = binary.operator()?.kind();
            if matches!(operator, RSyntaxKind::AND2 | RSyntaxKind::OR2) {
                // Short-circuit operators always return a scalar, even inside
                // `stopifnot()`, so their operands cannot pass as empty results.
                let left = condition_index_status(binary.left()?, name, truth, false)?;
                let right = || condition_index_status(binary.right()?, name, truth, false);
                // A false && or true || can result from either operand. Both
                // possibilities must support the same conclusion. For a true
                // && or false ||, either operand can establish it.
                let requires_both = matches!(
                    (operator, truth),
                    (RSyntaxKind::AND2, false) | (RSyntaxKind::OR2, true)
                );
                if requires_both {
                    return Ok(if left != IdentifierIndex::Unguarded && left == right()? {
                        left
                    } else {
                        IdentifierIndex::Unguarded
                    });
                }
                if left == IdentifierIndex::DefiniteZero {
                    return Ok(left);
                }
                return Ok(match right()? {
                    IdentifierIndex::Unguarded => left,
                    status => status,
                });
            }
            return comparison_index_status(&binary, name, truth, allow_empty);
        }
        _ => false,
    };
    Ok(if guarded {
        IdentifierIndex::Safe
    } else {
        IdentifierIndex::Unguarded
    })
}

/// Recognize comparisons of the variable or its length with a numeric literal.
/// A direct comparison can prove the index is zero or rule out zero. A length
/// comparison only tells us whether the index can be empty.
fn comparison_index_status(
    binary: &RBinaryExpression,
    name: &str,
    truth: bool,
    allow_empty: bool,
) -> anyhow::Result<IdentifierIndex> {
    let left = unparenthesized(binary.left()?)?;
    let right = unparenthesized(binary.right()?)?;
    let operator = binary.operator()?.kind();
    // Preserve operand order: `length(idx) > 1` becomes `0 > 1`, while
    // `1 < length(idx)` becomes `1 < 0`. The operator needs no reversal.
    let (expression, number, left_at_zero, right_at_zero) =
        if let Some(number) = numeric_literal(&right) {
            (left, number, 0.0, number)
        } else if let Some(number) = numeric_literal(&left) {
            (right, number, number, 0.0)
        } else {
            return Ok(IdentifierIndex::Unguarded);
        };
    let zero_result = match operator {
        RSyntaxKind::EQUAL2 => left_at_zero == right_at_zero,
        RSyntaxKind::NOT_EQUAL => left_at_zero != right_at_zero,
        RSyntaxKind::LESS_THAN => left_at_zero < right_at_zero,
        RSyntaxKind::LESS_THAN_OR_EQUAL_TO => left_at_zero <= right_at_zero,
        RSyntaxKind::GREATER_THAN => left_at_zero > right_at_zero,
        RSyntaxKind::GREATER_THAN_OR_EQUAL_TO => left_at_zero >= right_at_zero,
        _ => return Ok(IdentifierIndex::Unguarded),
    };
    // Direct vector comparisons can pass stopifnot() with an empty result.
    // `if` and a true `any()` require at least one result instead.
    if let AnyRExpression::RIdentifier(identifier) = expression {
        if allow_empty || identifier.name_text() != name {
            return Ok(IdentifierIndex::Unguarded);
        }
        let is_exact_zero = number == 0.0
            && matches!(
                (operator, truth),
                (RSyntaxKind::EQUAL2, true) | (RSyntaxKind::NOT_EQUAL, false)
            );
        return Ok(if is_exact_zero {
            IdentifierIndex::DefiniteZero
        } else if zero_result == truth {
            IdentifierIndex::Unguarded
        } else {
            IdentifierIndex::Safe
        });
    }
    Ok(
        if zero_result != truth
            && let AnyRExpression::RCall(call) = expression
            && is_index_length(&call, name)?
        {
            IdentifierIndex::Safe
        } else {
            IdentifierIndex::Unguarded
        },
    )
}

/// A true `any()` needs at least one true element. Only accept one data
/// argument: another argument could make it true while this index is empty.
fn is_any_guard(call: &RCall, name: &str) -> anyhow::Result<bool> {
    if !is_call_to(call, "any", "base")? {
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
    if !is_call_to(call, "stopifnot", "base")?
        || has_write(call.syntax(), name, call.range().end())?
    {
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
    Ok(is_call_to(call, "length", "base")? && has_single_variable_argument(call, name)?)
}

/// Discard a guard if the variable may have been assigned before its use.
/// Writes in nested function definitions do not run when defining the function.
/// When the scan stops inside `else`, the corresponding `then` branch cannot
/// run on that path. A scan through the end of a loop still considers both
/// branches because either can run on an earlier iteration.
fn has_write(node: &RSyntaxNode, name: &str, end: TextSize) -> anyhow::Result<bool> {
    let mut walk = node.preorder();
    let mut consequence_to_skip = None;
    while let Some(event) = walk.next() {
        let WalkEvent::Enter(node) = event else {
            continue;
        };
        if node.text_trimmed_range().start() >= end {
            break;
        }
        if consequence_to_skip.as_ref() == Some(&node) {
            walk.skip_subtree();
            consequence_to_skip = None;
            continue;
        }
        if let Some(statement) = RIfStatement::cast_ref(&node)
            && let Some(else_clause) = statement.else_clause()
            && let Ok(alternative) = else_clause.alternative()
            && alternative.range().contains(end)
            && let Ok(consequence) = statement.consequence()
        {
            consequence_to_skip = Some(consequence.into_syntax());
        }
        if RFunctionDefinition::can_cast(node.kind()) {
            walk.skip_subtree();
            continue;
        }
        // A for loop assigns its index variable on each iteration, which
        // invalidates earlier evidence even without an explicit assignment.
        if let Some(for_statement) = RForStatement::cast_ref(&node)
            && let AnyRForVariable::RIdentifier(identifier) = for_statement.variable()?
            && identifier.name_text() == name
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
