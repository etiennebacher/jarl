use crate::checker::{Checker, PackageOrigin};
use crate::diagnostic::*;
use crate::utils::{get_function_name, get_function_namespace_prefix, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for negations inside `dplyr::filter()` that can be replaced with
/// `dplyr::filter_out()`.
///
/// ## Why is this bad?
///
/// Using `filter()` with negated conditions can be hard to read, especially
/// when we also want to retain missing values. `filter(!condition)` drops rows
/// where `condition` is `TRUE` **and** rows where it is `NA`, meaning that if
/// we want to retain those then we have to complement the condition with
/// `is.na()`:
///
/// ```r
/// # We want to drop rows whose value for `col` is larger than the average
/// # of `col`:
/// larger_than_average <- function(x) x > mean(x, na.rm = TRUE)
/// x |> filter(!larger_than_average(col) | is.na(larger_than_average(col)))
/// ```
///
/// `dplyr` 1.2.0 introduced `filter_out()` as a complement to `filter()`.
/// `filter_out()` drops rows that match the condition, meaning that rows where
/// the condition is `NA` are retained. We can then rewrite the code above like
/// this:
///
/// ```r
/// x |> filter_out(larger_than_average(col))
/// ```
///
/// This rule suggests an automatic fix to rewrite them with `filter_out()`. It
/// is only valid for `dplyr` >= 1.2.0, and only works on `filter()` calls where
/// all conditions are made of one negation + `is.na()` on the same column.
///
/// ## Example
///
/// ```r
/// library(dplyr)
/// x <- tibble(a = c(1, 2, 2, NA), b = c(1, 1, 2, 3))
///
/// x |> filter(a > 1 | is.na(a))
///
/// x |> filter(a > 1 | is.na(a), is.na(b) | b <= 2)
/// ```
///
/// Use instead:
/// ```r
/// library(dplyr)
/// x <- tibble(a = c(1, 2, 2, NA), b = c(1, 1, 2, 3))
///
/// x |> filter_out(a <= 1)
///
/// x |> filter_out(a <= 1 | b > 2)
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
            PackageOrigin::Unknown => return Ok(None),
        }
    }

    // `filter_out()` was introduced in dplyr 1.2.0.
    if let Some(version) = checker.package_version("dplyr")
        && version < (1, 2, 0)
    {
        return Ok(None);
    }

    let args = ast.arguments()?;
    let items: Vec<_> = args.items().into_iter().collect();

    let mut unnamed_args: Vec<AnyRExpression> = Vec::new();
    let mut named_args: Vec<RArgument> = Vec::new();

    for item in &items {
        let arg = match item.as_ref() {
            Ok(a) => a,
            Err(_) => continue,
        };
        if let Some(arg_name) = arg.name_clause()
            && let Ok(arg_name) = arg_name.name()
        {
            // Named args other than ".by" and ".preserve" don't work in filter()
            let arg_name = arg_name.to_trimmed_text();
            if arg_name != ".by" && arg_name != ".preserve" {
                return Ok(None);
            }
            named_args.push(arg.clone());
        } else if let Some(value) = arg.value() {
            unnamed_args.push(value);
        }
    }

    if unnamed_args.is_empty() {
        return Ok(None);
    }

    // Extract the conditions from the `cond | is.na(var)` pattern and negate
    // them for `filter_out()`. Returns `None` if any condition doesn't match.
    let Some(conditions) = convert_conditions(&unnamed_args) else {
        return Ok(None);
    };

    let ns_prefix = fn_ns.as_deref().unwrap_or("");
    // Multiple comma-separated args in filter() are AND conditions, so we
    // join the rewritten conditions with OR.
    let joined_conds = conditions.join(" | ");

    let mut replacement_args = vec![joined_conds];
    for named in &named_args {
        replacement_args.push(named.syntax().text_trimmed().to_string());
    }

    let replacement = format!("{}filter_out({})", ns_prefix, replacement_args.join(", "));
    let range = ast.syntax().text_trimmed_range();

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            "dplyr_filter_out".to_string(),
            "This `| is.na()` pattern can be replaced by `filter_out()`.".to_string(),
            Some(
                "`filter_out()` keeps `NA` rows automatically, so the guard is unnecessary."
                    .to_string(),
            ),
        ),
        range,
        Fix {
            content: replacement,
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    )))
}

/// For each unnamed arg, extract the condition from a `cond | is.na(var)`
/// pattern and negate it for `filter_out()`.
///
/// Returns `None` if any argument doesn't match the pattern.
fn convert_conditions(args: &[AnyRExpression]) -> Option<Vec<String>> {
    let mut negated_conds: Vec<String> = Vec::new();

    for value in args {
        let (cond, is_na_call) = extract_is_na_guard(value)?;

        // Verify the is.na() argument appears in the condition.
        // This avoids matching `a > 1 | is.na(b)` where the guard is for
        // a different variable.
        let is_na_arg = extract_is_na_arg(&is_na_call)?;
        let cond_text = cond.syntax().text_trimmed().to_string();
        if !cond_text.contains(&is_na_arg) {
            return None;
        }

        negated_conds.push(negate_expression(&cond)?);
    }

    Some(negated_conds)
}

/// Extract the two sides of a `cond | is.na(var)` expression.
///
/// Returns `(condition, is_na_call)` in canonical order regardless of which
/// side `is.na()` appears on. Returns `None` if the expression doesn't match.
fn extract_is_na_guard(value: &AnyRExpression) -> Option<(AnyRExpression, AnyRExpression)> {
    let binary = value.as_r_binary_expression()?;
    let operator = binary.operator().ok()?;

    if operator.kind() != RSyntaxKind::OR {
        return None;
    }

    let left = binary.left().ok()?;
    let right = binary.right().ok()?;

    // Try both orientations: `cond | is.na(var)` and `is.na(var) | cond`
    if is_is_na_call(&right) {
        Some((left, right))
    } else if is_is_na_call(&left) {
        Some((right, left))
    } else {
        None
    }
}

/// Check if an expression is an `is.na(...)` call.
fn is_is_na_call(expr: &AnyRExpression) -> bool {
    expr.as_r_call()
        .and_then(|call| call.function().ok())
        .is_some_and(|f| get_function_name(f) == "is.na")
}

/// Extract the argument text from an `is.na(var)` call.
fn extract_is_na_arg(expr: &AnyRExpression) -> Option<String> {
    let call = expr.as_r_call()?;
    let args = call.arguments().ok()?;
    let first = args.items().into_iter().next()?.ok()?;
    let value = first.value()?;
    Some(value.syntax().text_trimmed().to_string())
}

/// Negate an expression for use in `filter_out()`.
///
/// Returns `None` for tidy eval expressions (`!!` / `!!!`).
///
/// - If already negated (`!expr`), strips the `!`
/// - Comparison operators are inverted: `a > 1` → `a <= 1`
/// - Simple identifiers/calls: `!expr`
/// - Complex expressions (binary, etc.): `!(expr)`
fn negate_expression(expr: &AnyRExpression) -> Option<String> {
    // If the expression is already `!something`, just unwrap it.
    // `extract_negated_inner` returns `None` for `!!` / `!!!` (tidy eval),
    // so we fall through to the tidy-eval guard below.
    if let Some(inner) = extract_negated_inner(expr) {
        return Some(inner);
    }

    // Reject tidy eval expressions (`!!x`, `!!!x`)
    if expr
        .as_r_unary_expression()
        .and_then(|u| u.operator().ok())
        .is_some_and(|op| op.kind() == RSyntaxKind::BANG)
    {
        return None;
    }

    // Invert comparison operators directly: `a > 1` → `a <= 1`
    if let Some(binary) = expr.as_r_binary_expression()
        && let Ok(op) = binary.operator()
    {
        let inv_op = match op.kind() {
            RSyntaxKind::GREATER_THAN => Some("<="),
            RSyntaxKind::GREATER_THAN_OR_EQUAL_TO => Some("<"),
            RSyntaxKind::LESS_THAN => Some(">="),
            RSyntaxKind::LESS_THAN_OR_EQUAL_TO => Some(">"),
            RSyntaxKind::EQUAL2 => Some("!="),
            RSyntaxKind::NOT_EQUAL => Some("=="),
            _ => None,
        };

        if let Some(inv_op) = inv_op
            && let (Ok(left), Ok(right)) = (binary.left(), binary.right())
        {
            return Some(format!(
                "{} {} {}",
                left.syntax().text_trimmed(),
                inv_op,
                right.syntax().text_trimmed()
            ));
        }
    }

    let text = expr.syntax().text_trimmed().to_string();

    // Simple expressions (identifiers, function calls) don't need parens
    let is_simple = expr.as_r_identifier().is_some()
        || expr.as_r_call().is_some()
        || expr.as_r_parenthesized_expression().is_some();

    if is_simple {
        Some(format!("!{text}"))
    } else {
        Some(format!("!({text})"))
    }
}

/// Extract the inner expression from a negated value (`!expr`).
///
/// Returns `None` if the value is not a single negation (e.g. `!!` or `!!!`
/// for tidy eval, or not negated at all).
///
/// Strips outer parentheses: `!(expr)` returns `"expr"`, not `"(expr)"`.
fn extract_negated_inner(value: &AnyRExpression) -> Option<String> {
    let unary = value.as_r_unary_expression()?;
    let operator = unary.operator().ok()?;
    if operator.kind() != RSyntaxKind::BANG {
        return None;
    }

    // Get the operand (skip BANG tokens)
    let operand = unary
        .syntax()
        .children()
        .find(|child| child.kind() != RSyntaxKind::BANG)?;

    // If the operand is itself a `!`, this is `!!` or `!!!`
    if RUnaryExpression::cast(operand.clone())
        .and_then(|u| u.operator().ok())
        .is_some_and(|op| op.kind() == RSyntaxKind::BANG)
    {
        return None;
    }

    // Strip outer parentheses: `!(expr)` → get inner node
    let inner = if operand.kind() == RSyntaxKind::R_PARENTHESIZED_EXPRESSION {
        operand.children().find(|child| {
            child.kind() != RSyntaxKind::L_PAREN && child.kind() != RSyntaxKind::R_PAREN
        })?
    } else {
        operand
    };

    Some(inner.text_trimmed().to_string())
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
