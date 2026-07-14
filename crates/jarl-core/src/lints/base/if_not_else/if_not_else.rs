use std::collections::HashSet;

use crate::checker::Checker;
use crate::diagnostic::*;
use crate::utils::{get_arg_by_position, get_function_name};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for `if` - `else` statements and `ifelse()` / `dplyr::if_else()` /
/// `data.table::fifelse()` calls whose condition is a simple negation, e.g.
/// `if (!cond) x else y` or `ifelse(!cond, x, y)`.
///
/// ## Why is this bad?
///
/// Negating the condition forces the reader to mentally flip the branches. It is
/// usually clearer to write the condition positively and swap the branches:
/// `if (A) y else x` instead of `if (!A) x else y`.
///
/// Negated calls such as `is.null()`, `is.na()` and `missing()` are common and
/// read naturally, so they are allowed by default. Use the `skipped-functions`
/// option to change this list.
///
/// This rule does not have an automatic fix.
///
/// ## Example
///
/// ```r
/// if (!cond) x else y
///
/// ifelse(!cond, x, y)
/// ```
///
/// Use instead:
///
/// ```r
/// if (cond) y else x
///
/// ifelse(cond, y, x)
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

    let skipped_functions = &checker.rule_options.if_not_else.skipped_functions;
    if !is_flaggable_negation(&ast.condition()?, skipped_functions)? {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "if_not_else".to_string(),
            "Negating the condition like `if (!A) y else x` can be hard to read.".to_string(),
            Some("Remove the negation and swap branches, such as `if (A) x else y`".to_string()),
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

    let skipped_functions = &checker.rule_options.if_not_else.skipped_functions;
    if !is_flaggable_negation(&condition, skipped_functions)? {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "if_not_else".to_string(),
            format!("Negating the condition like `{function_name}(!A, y, x)` can be hard to read."),
            Some(format!(
                "Remove the negation and swap branches, such as `{function_name}(A, x, y)`."
            )),
        ),
        range,
        Fix::empty(),
    );

    Ok(Some(diagnostic))
}

/// A condition is flaggable when its outer operator is a `!` negation, unless it
/// is a double negation (`!!A`) or a negated call to one of the
/// `skipped_functions` (e.g. `!is.null(x)`).
fn is_flaggable_negation(
    condition: &AnyRExpression,
    skipped_functions: &HashSet<String>,
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

    // Skip negated calls to skipped functions like `!is.null(x)`.
    if let AnyRExpression::RCall(call) = &argument {
        let name = get_function_name(call.function()?);
        if skipped_functions.contains(&name) {
            return Ok(false);
        }
    }

    Ok(true)
}
