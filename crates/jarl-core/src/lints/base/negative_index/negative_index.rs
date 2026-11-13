use crate::diagnostic::{Diagnostic, Fix, Violation};
use crate::rule_set::Rule;
use crate::utils::{get_function_name, get_function_namespace_prefix};
use air_r_syntax::*;
use biome_rowan::{AstNode, WalkEvent};

use super::nonempty_guard::{has_nonempty_guard, is_constant_numeric_expression};

pub struct NegativeIndex;

/// <!-- docs: start -->
/// Version added: 0.8.0
///
/// ## What it does
///
/// Checks for unary minus in `[]` indices, such as `x[-idx]` and `x[-which(x == 99)]`.
///
/// The rule will not report for cases below:
///
/// - Literals and constant arithmetic, such as `x[-1]`, `x[-2 * 3]`, and `x[c(-1, -2)]`.
/// - Negation inside comparisons, such as `x[x > -n]`.
/// - Negation inside `which()`, `order()`, and `between()`.
/// - `-idx` guarded by an `if` or `while` condition proving it is nonempty,
///   such as `length(idx) > 0`, `length(idx)`, or `any(idx > 0)`.
/// - `-idx` after a check that exits with `return()`, `stop()`, `next`, or `break`
///   when the index is empty, or an assertion such as `stopifnot(length(idx) > 0)`.
/// - `-idx` after a constant numeric assignment, such as `idx <- 1` or `idx <- 2 * 3`.
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
    for arg in ast.arguments()?.items() {
        let Some(index) = arg?.value() else {
            continue;
        };

        let mut expression = index.clone();
        while let AnyRExpression::RParenthesizedExpression(paren) = expression {
            expression = paren.body()?;
        }

        // Check the whole index: a constant part of `x[-2 * idx]` cannot
        // establish that the complete expression is independent of runtime values.
        if is_literal(expression.clone())? {
            continue;
        }

        // A guard protects the whole index only for a direct negation.
        // In `x[-n * idx]`, checking `n` does not establish `idx`'s length.
        if let AnyRExpression::RUnaryExpression(unary) = expression
            && unary.operator()?.kind() == RSyntaxKind::MINUS
            && has_nonempty_guard(&unary)?
        {
            continue;
        }

        if contains_negative(index.syntax())? {
            diagnostics.push(Diagnostic::new(
                NegativeIndex,
                index.syntax().text_trimmed_range(),
                Fix::empty(),
            ));
        }
    }
    Ok(diagnostics)
}

/// Find negation that can contribute negative values to this index.
fn contains_negative(node: &RSyntaxNode) -> anyhow::Result<bool> {
    let mut preorder = node.preorder();
    while let Some(event) = preorder.next() {
        let WalkEvent::Enter(node) = event else {
            continue;
        };

        // Nested indices are checked separately, as in `x[y[-idx]]`, but their
        // objects still contribute to this index, as in `x[(-idx)[1]]`.
        // Function bodies, as in `x[f(function() -idx)]`, do not compute it.
        if RSubsetArguments::can_cast(node.kind())
            || RSubset2Arguments::can_cast(node.kind())
            || RFunctionDefinition::can_cast(node.kind())
        {
            preorder.skip_subtree();
            continue;
        }

        let skip_children = match AnyRExpression::cast(node) {
            // Comparisons and logical operators produce logical indices, so
            // negative values in their operands cannot pass through.
            Some(AnyRExpression::RBinaryExpression(binary)) => matches!(
                binary.operator()?.kind(),
                RSyntaxKind::EQUAL2
                    | RSyntaxKind::NOT_EQUAL
                    | RSyntaxKind::LESS_THAN
                    | RSyntaxKind::LESS_THAN_OR_EQUAL_TO
                    | RSyntaxKind::GREATER_THAN
                    | RSyntaxKind::GREATER_THAN_OR_EQUAL_TO
                    | RSyntaxKind::AND
                    | RSyntaxKind::AND2
                    | RSyntaxKind::OR
                    | RSyntaxKind::OR2
            ),
            Some(AnyRExpression::RUnaryExpression(unary)) => match unary.operator()?.kind() {
                RSyntaxKind::BANG => true,
                // Inspect the outer minus before skipping its operand, so
                // `x[-which(x)]` and `x[-(x > -n)]` are still reported.
                RSyntaxKind::MINUS => return Ok(true),
                _ => false,
            },
            Some(AnyRExpression::RCall(call)) => should_skip_call(&call)?,
            _ => false,
        };
        if skip_children {
            preorder.skip_subtree();
        }
    }
    Ok(false)
}

/// Skip calls whose arguments cannot contribute runtime-dependent negative
/// indices:
/// - `which()` and `order()` return only positive indices
/// - `dplyr::between()` and `data.table::between()` return only logical values
/// - `c()` with only literal arguments returns only literal values
fn should_skip_call(call: &RCall) -> anyhow::Result<bool> {
    let function = call.function()?;
    // Only assume these semantics for unqualified calls and known namespaces.
    let skips_negative_arguments = matches!(
        (
            get_function_name(function.clone()).as_str(),
            get_function_namespace_prefix(function).as_deref()
        ),
        ("which" | "order", None | Some("base::"))
            | ("between", None | Some("dplyr::" | "data.table::"))
    );
    Ok(skips_negative_arguments || is_literal_c_call(call)?)
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

fn is_literal(mut expression: AnyRExpression) -> anyhow::Result<bool> {
    while let AnyRExpression::RParenthesizedExpression(paren) = expression {
        expression = paren.body()?;
    }
    match expression {
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
