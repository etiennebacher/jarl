use std::collections::HashSet;

use crate::checker::Checker;
use crate::diagnostic::*;
use crate::utils::{get_arg_by_position, get_function_name};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// Version added: 0.4.0
///
/// ## What it does
///
/// Checks for `if`/`else` statements and `ifelse()` calls whose condition is a
/// simple negation, e.g. `if (!A) x else y` or `ifelse(!A, x, y)`.
///
/// This also covers the data.table (`fifelse()`) and dplyr (`if_else()`)
/// equivalents of `ifelse()`.
///
/// ## Why is this bad?
///
/// Negating the condition forces the reader to mentally flip the branches. It is
/// usually clearer to write the condition positively and swap the branches:
/// `if (A) y else x` instead of `if (!A) x else y`.
///
/// Negated calls such as `is.null()`, `is.na()` and `missing()` are common and
/// read naturally, so they are allowed by default. Use the `exceptions` option
/// to change this list.
///
/// This rule does not have an automatic fix.
///
/// ## Example
///
/// ```r
/// if (!A) x else y
///
/// ifelse(!A, x, y)
/// ```
///
/// Use instead:
///
/// ```r
/// if (A) y else x
///
/// ifelse(A, y, x)
/// ```
pub fn if_not_else(ast: &RIfStatement, checker: &Checker) -> anyhow::Result<Option<Diagnostic>> {
    // Only simple `if`/`else` statements, not `if`/`else if` chains: swapping the
    // branches of an `else if` wouldn't be a straightforward rewrite.
    let Some(else_clause) = ast.else_clause() else {
        return Ok(None);
    };
    let alternative = else_clause.alternative()?;
    if matches!(alternative, AnyRExpression::RIfStatement(_)) {
        return Ok(None);
    }

    let exceptions = &checker.rule_options.if_not_else.exceptions;
    if !is_flaggable_negation(&ast.condition()?, exceptions)? {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "if_not_else".to_string(),
            "Prefer `if (A) x else y` to the less-readable `if (!A) y else x` in a simple if/else statement.".to_string(),
            None,
        ),
        range,
        Fix::empty(),
    );

    Ok(Some(diagnostic))
}

/// Handle the `ifelse()`/`fifelse()`/`if_else()` variants of the same pattern.
pub fn if_not_else_call(ast: &RCall, checker: &Checker) -> anyhow::Result<Option<Diagnostic>> {
    let function_name = get_function_name(ast.function()?);
    if !matches!(function_name.as_str(), "ifelse" | "fifelse" | "if_else") {
        return Ok(None);
    }

    let args = ast.arguments()?.items();
    let Some(first_arg) = get_arg_by_position(&args, 1) else {
        return Ok(None);
    };
    let Some(condition) = first_arg.value() else {
        return Ok(None);
    };

    let exceptions = &checker.rule_options.if_not_else.exceptions;
    if !is_flaggable_negation(&condition, exceptions)? {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "if_not_else".to_string(),
            format!(
                "Prefer `{function_name}(A, x, y)` to the less-readable `{function_name}(!A, y, x)`."
            ),
            None,
        ),
        range,
        Fix::empty(),
    );

    Ok(Some(diagnostic))
}

/// A condition is flaggable when its outer operator is a `!` negation, unless it
/// is a double negation (`!!A`) or a negated call to one of the `exceptions`
/// (e.g. `!is.null(x)`).
fn is_flaggable_negation(
    condition: &AnyRExpression,
    exceptions: &HashSet<String>,
) -> anyhow::Result<bool> {
    let AnyRExpression::RUnaryExpression(unary) = condition else {
        return Ok(false);
    };
    if unary.operator()?.text_trimmed() != "!" {
        return Ok(false);
    }

    let argument = unary.argument()?;

    // Skip double negation like `!!A`.
    if matches!(argument, AnyRExpression::RUnaryExpression(_)) {
        return Ok(false);
    }

    // Skip negated calls to excepted functions like `!is.null(x)`.
    if let AnyRExpression::RCall(call) = &argument {
        let name = get_function_name(call.function()?);
        if exceptions.contains(&name) {
            return Ok(false);
        }
    }

    Ok(true)
}
