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
/// `filter(!condition)` drops rows where `condition` is `TRUE` **and** rows
/// where it is `NA`. `filter_out(condition)` drops only `TRUE` rows, keeping
/// `NA`s. Using `filter_out()` avoids accidentally dropping `NA` rows and
/// removes the need for verbose `| is.na()` guards.
///
/// ## Details
///
/// `filter_out()` was introduced in dplyr 1.2.0.
///
/// Note that `filter(!cond)` and `filter_out(cond)` handle `NA` values
/// differently: `filter()` drops `NA` rows while `filter_out()` keeps them.
/// The automatic fix is only applied for the `cond | is.na(var)` pattern,
/// where the replacement is semantically equivalent. For plain negations
/// (`filter(!cond)`), only a diagnostic is emitted.
///
/// ## Example
///
/// ```r
/// library(dplyr)
/// x <- tibble(a = c(1, 2, 2, NA), b = c(1, 1, 2, 3))
///
/// x |> filter(a > 1 | is.na(a))
///
/// x |> filter(a > 1 | is.na(a), b < 2)
/// ```
///
/// Use instead:
/// ```r
/// library(dplyr)
/// x <- tibble(a = c(1, 2, NA))
///
/// x |> filter_out(a <= 1)
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

    // `filter_out()` was introduced in dplyr 1.2.0. Skip if the installed
    // version is older.
    if let Some(version) = checker.package_version("dplyr")
        && version < (1, 2, 0)
    {
        return Ok(None);
    }

    let args = ast.arguments()?;
    let items: Vec<_> = args.items().into_iter().collect();

    // Separate unnamed (filtering) args from named args
    let mut unnamed_args: Vec<AnyRExpression> = Vec::new();
    let mut named_args: Vec<RArgument> = Vec::new();

    for item in &items {
        let arg = match item.as_ref() {
            Ok(a) => a,
            Err(_) => continue,
        };
        if arg.name_clause().is_some() {
            named_args.push(arg.clone());
        } else if let Some(value) = arg.value() {
            unnamed_args.push(value);
        }
    }

    if unnamed_args.is_empty() {
        return Ok(None);
    }

    // Try the `cond | is.na(var)` pattern first (safe fix available).
    if let Some(diagnostic) = check_is_na_guard_pattern(ast, &fn_ns, &unnamed_args, &named_args) {
        return Ok(Some(diagnostic));
    }

    // Fall back to plain negation pattern (no auto-fix).
    check_negation_pattern(ast, &unnamed_args)
}

/// Detect `filter(cond | is.na(var), ...)` and offer a safe fix to
/// `filter_out(!cond, ...)`.
///
/// This is semantically equivalent because `filter_out()` already keeps
/// `NA` rows, so the explicit `| is.na()` guard is redundant.
fn check_is_na_guard_pattern(
    ast: &RCall,
    fn_ns: &Option<String>,
    unnamed_args: &[AnyRExpression],
    named_args: &[RArgument],
) -> Option<Diagnostic> {
    // All unnamed args must match `cond | is.na(var)`
    let mut negated_conds: Vec<String> = Vec::new();

    for value in unnamed_args {
        let cond_text = extract_is_na_guard(value)?;
        negated_conds.push(cond_text);
    }

    let ns_prefix = fn_ns.as_deref().unwrap_or("");
    // Multiple comma-separated args in filter() are AND conditions, so we
    // should join the rewritten conditions with OR.
    let filter_out_cond = negated_conds.join(" | ");

    let mut replacement_args = vec![filter_out_cond];
    for named in named_args {
        replacement_args.push(named.syntax().text_trimmed().to_string());
    }

    let replacement = format!("{}filter_out({})", ns_prefix, replacement_args.join(", "));
    let range = ast.syntax().text_trimmed_range();

    Some(Diagnostic::new(
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
    ))
}

/// Extract the condition from a `cond | is.na(var)` expression.
///
/// Returns the negated condition text (prefixed with `!`) suitable for
/// `filter_out()`. Returns `None` if the expression doesn't match.
fn extract_is_na_guard(value: &AnyRExpression) -> Option<String> {
    let binary = value.as_r_binary_expression()?;
    let operator = binary.operator().ok()?;

    if operator.kind() != RSyntaxKind::OR {
        return None;
    }

    let left = binary.left().ok()?;
    let right = binary.right().ok()?;

    // Try both orientations: `cond | is.na(var)` and `is.na(var) | cond`
    let (cond, is_na_call) = if is_is_na_call(&right) {
        (left, right)
    } else if is_is_na_call(&left) {
        (right, left)
    } else {
        return None;
    };

    // Verify the is.na() argument appears in the condition.
    // This avoids matching `a > 1 | is.na(b)` where the guard is for
    // a different variable.
    let is_na_arg = extract_is_na_arg(&is_na_call)?;
    let cond_text = cond.syntax().text_trimmed().to_string();
    if !cond_text.contains(&is_na_arg) {
        return None;
    }

    // Negate the condition for filter_out().
    // Simple expressions get `!expr`, complex ones get `!(expr)`.
    let negated = negate_expression(&cond);
    Some(negated)
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
/// - If already negated (`!expr`), strips the `!`
/// - Comparison operators are inverted: `a > 1` → `a <= 1`
/// - Simple identifiers/calls: `!expr`
/// - Complex expressions (binary, etc.): `!(expr)`
fn negate_expression(expr: &AnyRExpression) -> String {
    // If the expression is already `!something`, just unwrap it
    if let Some(inner) = extract_negated_inner(expr) {
        return inner;
    }

    // Invert comparison operators directly: `a > 1` → `a <= 1`
    if let Some(binary) = expr.as_r_binary_expression()
        && let Ok(op) = binary.operator()
    {
        let inverted = match op.kind() {
            RSyntaxKind::GREATER_THAN => Some("<="),
            RSyntaxKind::GREATER_THAN_OR_EQUAL_TO => Some("<"),
            RSyntaxKind::LESS_THAN => Some(">="),
            RSyntaxKind::LESS_THAN_OR_EQUAL_TO => Some(">"),
            RSyntaxKind::EQUAL2 => Some("!="),
            RSyntaxKind::NOT_EQUAL => Some("=="),
            _ => None,
        };

        if let Some(inv_op) = inverted
            && let (Ok(left), Ok(right)) = (binary.left(), binary.right())
        {
            return format!(
                "{} {} {}",
                left.syntax().text_trimmed(),
                inv_op,
                right.syntax().text_trimmed()
            );
        }
    }

    let text = expr.syntax().text_trimmed().to_string();

    // Simple expressions (identifiers, function calls) don't need parens
    let is_simple = expr.as_r_identifier().is_some()
        || expr.as_r_call().is_some()
        || expr.as_r_parenthesized_expression().is_some();

    if is_simple {
        format!("!{text}")
    } else {
        format!("!({text})")
    }
}

/// Detect `filter(!cond, ...)` where all unnamed args are negated.
///
/// Emits a diagnostic without an auto-fix because `filter(!cond)` and
/// `filter_out(cond)` handle `NA`s differently.
fn check_negation_pattern(
    ast: &RCall,
    unnamed_args: &[AnyRExpression],
) -> anyhow::Result<Option<Diagnostic>> {
    for value in unnamed_args {
        if extract_negated_inner(value).is_none() {
            return Ok(None);
        }
    }

    let range = ast.syntax().text_trimmed_range();

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            "dplyr_filter_out".to_string(),
            "Negating conditions in `filter()` can be hard to read.".to_string(),
            Some("You could use `filter_out()` instead (but beware of `NA` handling).".to_string()),
        ),
        range,
        Fix::empty(),
    )))
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

    // Strip outer parentheses: `!(expr)` → `expr`
    let text = if operand.kind() == RSyntaxKind::R_PARENTHESIZED_EXPRESSION {
        operand
            .children()
            .find(|child| {
                child.kind() != RSyntaxKind::L_PAREN && child.kind() != RSyntaxKind::R_PAREN
            })
            .map(|child| child.text_trimmed().to_string())
            .unwrap_or_else(|| operand.text_trimmed().to_string())
    } else {
        operand.text_trimmed().to_string()
    };

    Some(text)
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
