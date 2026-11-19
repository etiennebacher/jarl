use crate::diagnostic::{Diagnostic, Fix, Violation};
use crate::rule_set::Rule;
use crate::utils::get_function_name;
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};
use oak_core::syntax_ext::RIdentifierExt;

use super::nonempty_guard::{
    has_nonempty_guard, is_constant_numeric_expression, is_index_length, unparenthesized,
};

pub struct NegativeIndex;

/// <!-- docs: start -->
/// Version added: 0.8.0
///
/// ## What it does
///
/// Checks for `[]` indices whose outermost operator is unary minus, such as
/// `x[-idx]` and `x[-which(x == 99)]`. Parentheses around the index are ignored.
/// A minus inside a call or multiplication, as in `x[head(idx, -1)]` or
/// `x[-1 * idx]`, is outside the scope of this rule.
///
/// The rule will not report for cases below:
///
/// - Literals and constant arithmetic, such as `x[-1]`, `x[-(2 * 3)]`, and `x[-c(1, 2)]`.
/// - A single index such as `x[-length(x)]`, where the same variable is subsetted
///   and passed to `length()`. If `x` is empty, selecting nothing is expected.
/// - `-idx` guarded by an `if` or `while` condition proving it is nonempty,
///   such as `length(idx) > 0`, `length(idx)`, or `any(idx > 0)`.
/// - `-idx` after a check that exits when the index is empty, for example with
///   `return()`, `stop()`, `rlang::abort()`, `cli::cli_abort()`, `next`, or `break`,
///   or after an assertion such as `stopifnot(length(idx) > 0)`.
/// - `-idx` after a constant numeric assignment, such as `idx <- 1` or `idx <- 2 * 3`.
///   Assigning a zero literal still triggers a warning on `x[-idx]` because `x[-0]`
///   selects nothing.
///
/// ## Why is this bad?
///
/// If `idx` is empty, `x[-idx]` selects no elements instead of keeping all of
/// `x`. For example, trying to remove a value that is not present returns an
/// empty vector:
///
/// ```r
/// x <- 1:10
/// x[-which(x == 99)]
/// #> integer(0)
/// ```
///
/// Since `99` is not in `x`, the intended result is the unchanged vector `1:10`
/// rather than an empty vector.
///
/// No automatic fix is provided. Handle empty indices explicitly or use a
/// logical index when possible.
///
/// ## Example
///
/// ```r
/// x <- 1:10
/// x[-which(x == 99)]
/// ```
///
/// Use instead:
///
/// ```r
/// x <- 1:10
/// x[x != 99]
/// ```
/// <!-- docs: end -->
impl Violation for NegativeIndex {
    fn rule(&self) -> Rule {
        Rule::NegativeIndex
    }

    fn body(&self) -> String {
        "Non-literal negative indices may produce unexpected results when empty.".to_string()
    }

    fn suggestion(&self) -> Option<String> {
        Some("Handle empty indices explicitly or use a logical index.".to_string())
    }
}

pub fn negative_index(ast: &RSubset) -> anyhow::Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let arguments = ast.arguments()?.items();
    let single_index = arguments.len() == 1;
    for arg in arguments {
        let arg = arg?;
        let Some(index) = arg.value() else {
            continue;
        };

        let AnyRExpression::RUnaryExpression(unary) = unparenthesized(index.clone())? else {
            continue;
        };
        if unary.operator()?.kind() != RSyntaxKind::MINUS
            || is_literal(unary.argument()?)?
            || (single_index && arg.name_clause().is_none() && is_subset_length(ast, &unary)?)
            || has_nonempty_guard(&unary)?
        {
            continue;
        }

        diagnostics.push(Diagnostic::new(
            NegativeIndex,
            index.syntax().text_trimmed_range(),
            Fix::empty(),
        ));
    }
    Ok(diagnostics)
}

/// Recognize `x[-length(x)]` by comparing variable names. Repeated calls such
/// as `f()[-length(f())]` can produce different objects.
fn is_subset_length(subset: &RSubset, unary: &RUnaryExpression) -> anyhow::Result<bool> {
    let AnyRExpression::RCall(call) = unparenthesized(unary.argument()?)? else {
        return Ok(false);
    };
    let AnyRExpression::RIdentifier(object) = unparenthesized(subset.function()?)? else {
        return Ok(false);
    };
    is_index_length(&call, &object.name_text())
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
