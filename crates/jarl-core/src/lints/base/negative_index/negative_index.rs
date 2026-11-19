use crate::diagnostic::{Diagnostic, Fix, ViolationData};
use crate::rule_set::Rule;
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};
use oak_core::syntax_ext::RIdentifierExt;

use super::literal_index::{LiteralIndex, is_literal_index};
use super::nonempty_guard::{has_nonempty_guard, is_index_length};

/// <!-- docs: start -->
/// Version added: 0.8.0
///
/// ## What it does
///
/// Checks for `[]` indices whose outermost operator is unary minus `-`, such as
/// `x[-idx]` and `x[-which(x == 99)]`.
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
            || (single_index && arg.name_clause().is_none() && is_subset_length(ast, &unary)?)
        {
            continue;
        }

        let (message, suggestion) = match is_literal_index(&unary)? {
            LiteralIndex::Literal => continue,
            LiteralIndex::Zero => (
                "The negated index is zero, so it selects no elements.",
                "Handle zero indices explicitly or use a logical index.",
            ),
            LiteralIndex::NonLiteral => {
                if has_nonempty_guard(&unary)? {
                    continue;
                }
                (
                    "Non-literal negative indices may produce unexpected results when empty.",
                    "Handle empty indices explicitly or use a logical index.",
                )
            }
        };
        diagnostics.push(Diagnostic::new(
            ViolationData::new(
                Rule::NegativeIndex,
                message.to_string(),
                Some(suggestion.to_string()),
            ),
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

/// Strip only the parentheses around the whole expression, so `((-idx))`
/// is checked as `-idx` while `-(idx)` keeps its parenthesized operand.
pub(super) fn unparenthesized(mut expression: AnyRExpression) -> anyhow::Result<AnyRExpression> {
    while let AnyRExpression::RParenthesizedExpression(paren) = expression {
        expression = paren.body()?;
    }
    Ok(expression)
}
