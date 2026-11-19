use crate::diagnostic::{Diagnostic, Fix, ViolationData};
use crate::rule_set::Rule;
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};
use oak_core::syntax_ext::RIdentifierExt;

use super::literal_index::{
    call_name_and_package, has_single_variable_argument, is_literal, is_zero_index, unparenthesized,
};
use super::nonempty_guard::{IdentifierIndex, check_identifier_index};

/// <!-- docs: start -->
/// Version added: 0.8.0
///
/// ## What it does
///
/// Checks for `[]` subsetting with non-literal negative indices, such as
/// variables or calls (`x[-idx]`, `x[-which(x == 99)]`). And also for
/// negated zero indices (`x[-0]`, or `x[-idx]` when `idx` is assigned `0`).
///
/// This rule does not report:
///
/// - Named arguments in `[]`, such as `roll = -tol` in `dt[x, roll = -tol]`.
/// - Non-zero literals, constant arithmetic, and `:` sequences, such as `x[-1]`,
///   `x[-(2 * 3)]`, `x[-c(1, 2)]`, and `x[-(1:n)]`.
/// - Calls to `c()` with a known positive subscript, such as `x[-c(idx, 1)]`,
///   `x[-c(idx, c(1, 2))]`, or `x[-c(idx, 1:3)]`.
/// - Subsetting by the same object's length or dimension, such as
///   `x[-length(x)]`, `df[-nrow(df), ]`, `df[, -ncol(df)]`, or `df[-NCOL(df)]`.
/// - Group size indices, such as `x[-n()]`, `x[-dplyr::n()]`, and `dt[-.N, ]`.
///   `n()` must have no arguments and be the sole index. `.N` must be the sole
///   index or the row index.
/// - An unchanged `for` loop variable drawn from `seq_along()`, `seq_len()`,
///   `which()`, or a range of positive integer literals such as `1:10`.
/// - `-idx` guarded by an `if` or `while` condition proving `idx` is nonempty,
///   such as `length(idx) > 0`, `length(idx)`, or `any(idx > 0)`.
/// - `-idx` after a guard that exits when `idx` is empty (such as `return()`,
///   `stop()`, `rlang::abort()`, `cli::cli_abort()`, `next`, or `break`), or
///   after an assertion such as `stopifnot(length(idx) > 0)`.
/// - `-idx` after assigning a non-zero constant or a `:` sequence, such as
///   `idx <- 1`, `idx <- 2 * 3`, or `idx <- a:b`.
///
/// ## Why is this bad?
///
/// If `idx` is empty or `0`, `x[-idx]` selects no elements instead of keeping
/// all of `x`. For example, trying to remove a value that is not present
/// returns an empty vector:
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
    let positional_arguments = || {
        arguments
            .iter()
            .filter(|arg| !arg.as_ref().is_ok_and(|arg| arg.name_clause().is_some()))
    };
    let index_count = positional_arguments().count();
    for (position, arg) in positional_arguments().enumerate() {
        let arg = arg?;
        let Some(index) = arg.value() else {
            continue;
        };

        let AnyRExpression::RUnaryExpression(unary) = unparenthesized(index.clone())? else {
            continue;
        };
        if unary.operator()?.kind() != RSyntaxKind::MINUS {
            continue;
        }

        let operand = unparenthesized(unary.argument()?)?;
        if is_subset_extent(ast, &operand, position, index_count)? {
            continue;
        }

        let zero_index = match &operand {
            AnyRExpression::RIdentifier(identifier) => {
                match check_identifier_index(&unary, identifier)? {
                    IdentifierIndex::Safe => continue,
                    IdentifierIndex::DefiniteZero => true,
                    IdentifierIndex::Unguarded => {
                        if position == 0 && index_count <= 2 && identifier.name_text() == ".N" {
                            continue;
                        }
                        false
                    }
                }
            }
            _ => {
                let zero = is_zero_index(&operand)?;
                if !zero && is_literal(&operand)? {
                    continue;
                }
                zero
            }
        };
        let (message, suggestion) = if zero_index {
            (
                "The negated index is zero, so it selects no elements.",
                "Handle zero indices explicitly or use a logical index.",
            )
        } else {
            (
                "Non-literal negative indices may produce unexpected results when empty or zero.",
                "Handle empty or zero indices explicitly, or use a logical index.",
            )
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

/// Match a group size or the extent of the object along the indexed dimension.
/// Restrict extent checks to variables: repeated calls can return different objects.
fn is_subset_extent(
    subset: &RSubset,
    operand: &AnyRExpression,
    position: usize,
    index_count: usize,
) -> anyhow::Result<bool> {
    let AnyRExpression::RCall(call) = operand else {
        return Ok(false);
    };
    let Some((name, package)) = call_name_and_package(call)? else {
        return Ok(false);
    };
    if name == "n" && index_count == 1 && matches!(package.as_deref(), None | Some("dplyr")) {
        return Ok(call.arguments()?.items().is_empty());
    }
    if !matches!(package.as_deref(), None | Some("base"))
        || !matches!(
            (index_count, position, name.as_str()),
            (1, 0, "length" | "ncol" | "NCOL") | (2, 0, "nrow" | "NROW") | (2, 1, "ncol" | "NCOL")
        )
    {
        return Ok(false);
    }
    let AnyRExpression::RIdentifier(object) = unparenthesized(subset.function()?)? else {
        return Ok(false);
    };
    has_single_variable_argument(call, &object.name_text())
}
