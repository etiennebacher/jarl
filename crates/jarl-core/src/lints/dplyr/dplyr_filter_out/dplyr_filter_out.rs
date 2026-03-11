use crate::checker::{Checker, PackageOrigin};
use crate::diagnostic::*;
use crate::utils::{get_function_name, get_function_namespace_prefix};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for negations inside `dplyr::filter()` that can be replaced with
/// `dplyr::filter_out()`.
///
/// ## Why is this bad?
///
/// `filter(!condition)` drops rows where `condition` is `TRUE` **and** rows
/// where it is `NA`. `filter_out(condition)` drops only `TRUE` rows, keeping
/// `NA`s. The intent is usually to keep `NA` rows, making `filter_out()` both
/// clearer and more correct.
///
/// ## Details
///
/// `filter_out()` was introduced in dplyr 1.2.0.
///
/// ## Example
///
/// ```r
/// library(dplyr)
/// x |> filter(!is.na(val))
/// ```
///
/// Use instead:
/// ```r
/// library(dplyr)
/// x |> filter_out(is.na(val))
/// ```
///
/// ## References
///
/// - <https://dplyr.tidyverse.org/reference/filter.html>
pub fn dplyr_filter_out(ast: &RCall, checker: &Checker) -> anyhow::Result<Option<Diagnostic>> {
    let fn_name = get_function_name(ast.function()?);
    let fn_ns = get_function_namespace_prefix(ast.function()?);

    // Only trigger on `filter()` or `dplyr::filter()`
    if fn_name != "filter" {
        return Ok(None);
    }
    if let Some(ref ns) = fn_ns
        && ns != "dplyr::"
    {
        return Ok(None);
    }

    // Without an explicit namespace, use the package cache to resolve
    // the package, falling back to requiring a pipe (which makes it
    // unlikely to be `stats::filter()`).
    if fn_ns.is_none() {
        match checker.resolve_package("filter") {
            PackageOrigin::Resolved(ref pkg) if pkg == "dplyr" => {}
            PackageOrigin::Resolved(_) => return Ok(None),
            // Multiple packages export `filter` (e.g. stats and dplyr).
            // Use pipe as a heuristic: piped `filter()` is likely dplyr.
            PackageOrigin::Ambiguous(_) => {
                if !checker.resolve_package("filter").includes("dplyr") || !is_piped_into(ast) {
                    return Ok(None);
                }
            }
            PackageOrigin::Unknown if !is_piped_into(ast) => return Ok(None),
            PackageOrigin::Unknown => {}
        }
    }

    // `dplyr_filter_out()` was introduced in dplyr 1.2.0. Skip if the installed
    // version is older.
    if let Some(version) = checker.package_version("dplyr")
        && version < (1, 2, 0)
    {
        return Ok(None);
    }

    let args = ast.arguments()?;
    let items: Vec<_> = args.items().into_iter().collect();

    // Look for any unnamed argument that is a `!expr` negation
    let negated_arg = items.iter().find_map(|item| {
        let arg = item.as_ref().ok()?;
        // Skip named arguments (e.g., `.by = grp`)
        if arg.name_clause().is_some() {
            return None;
        }
        let value = arg.value()?;
        // Check if the argument is a single `!expr` negation.
        // Skip `!!` and `!!!` (tidy eval injection operators).
        let unary = value.as_r_unary_expression()?;
        let operator = unary.operator().ok()?;
        if operator.kind() != RSyntaxKind::BANG {
            return None;
        }
        // If the operand is itself a `!`, this is `!!` or `!!!`
        let operand = unary
            .syntax()
            .children()
            .find(|child| child.kind() != RSyntaxKind::BANG)?;
        if RUnaryExpression::cast(operand)
            .and_then(|u| u.operator().ok())
            .is_some_and(|op| op.kind() == RSyntaxKind::BANG)
        {
            return None;
        }
        Some(unary.clone())
    });

    let Some(negated) = negated_arg else {
        return Ok(None);
    };

    // Get the inner expression (the part after `!`)
    let Some(inner_expr) = negated
        .syntax()
        .children()
        .find(|child| child.kind() != RSyntaxKind::BANG)
    else {
        return Ok(None);
    };

    // Strip outer parentheses: `!(expr)` → show `expr`, not `(expr)`
    let inner_text = if inner_expr.kind() == RSyntaxKind::R_PARENTHESIZED_EXPRESSION {
        inner_expr
            .children()
            .find(|child| {
                child.kind() != RSyntaxKind::L_PAREN && child.kind() != RSyntaxKind::R_PAREN
            })
            .map(|child| child.text_trimmed().to_string())
            .unwrap_or_else(|| inner_expr.text_trimmed().to_string())
    } else {
        inner_expr.text_trimmed().to_string()
    };
    let range = ast.syntax().text_trimmed_range();

    let body = "Negating conditions in `filter()` can be hard to read.".to_string();
    let suggestion = format!("Use `filter_out({inner_text})` instead.",);

    Ok(Some(Diagnostic::new(
        ViolationData::new("dplyr_filter_out".to_string(), body, Some(suggestion)),
        range,
        Fix::empty(),
    )))
}

/// Check if a call node receives input from a pipe (i.e., is on the right side).
fn is_piped_into(call: &RCall) -> bool {
    call.syntax()
        .prev_sibling_or_token()
        .map(|prev| {
            prev.kind() == RSyntaxKind::PIPE
                || (prev.kind() == RSyntaxKind::SPECIAL
                    && prev.as_token().is_some_and(|t| t.text_trimmed() == "%>%"))
        })
        .unwrap_or(false)
}
