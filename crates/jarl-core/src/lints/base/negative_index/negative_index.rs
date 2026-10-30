use crate::diagnostic::{Diagnostic, Fix, Violation};
use crate::rule_set::Rule;
use crate::utils::{get_function_name, get_function_namespace_prefix};
use air_r_syntax::*;
use biome_rowan::{AstNode, WalkEvent};

pub struct NegativeIndex;

/// <!-- docs: start -->
/// Version added: 0.8.0
///
/// ## What it does
///
/// Checks for unary minus in `[]` indices, such as `x[-idx]`, except for
/// directly negated literals such as `x[-1]`.
/// A `c()` call is also treated as literal when all its arguments are literals.
/// Negation inside logical expressions or calls to `between()`, `which()`, and
/// `order()` is ignored, as in `x[x > -1]` and `x[order(-x)]`.
///
/// ## Why is this bad?
///
/// If `idx` is empty, `x[-idx]` selects no elements instead of keeping all of
/// `x`. This can happen with `x[-which(condition)]` when nothing matches.
///
/// This rule also reports expressions such as `x[-1 * idx]`, `x[c(-1, -idx)]`,
/// and `x[-2 * 3]`. It does not determine whether an index is actually empty or
/// whether a surrounding condition already checks its length.
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

        if let AnyRExpression::RUnaryExpression(unary) = expression
            && unary.operator()?.kind() == RSyntaxKind::MINUS
            && is_literal(unary.argument()?)?
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

        match AnyRExpression::cast(node) {
            // Comparisons and logical operators produce logical values, not
            // negative indices: `x[x > -n]`, `x[!(-idx)]`, `x[a & -b]`.
            Some(AnyRExpression::RBinaryExpression(binary))
                if matches!(
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
                ) =>
            {
                preorder.skip_subtree();
            }
            Some(AnyRExpression::RUnaryExpression(unary)) => match unary.operator()?.kind() {
                RSyntaxKind::BANG => preorder.skip_subtree(),
                // Visiting the outer minus first still reports `x[-which(x)]`
                // and `x[-(x > -n)]`, even though their operands are skipped.
                RSyntaxKind::MINUS => return Ok(true),
                _ => {}
            },
            Some(AnyRExpression::RCall(call)) => {
                let function = call.function()?;
                // Position-generating calls and `between()` do not pass negative
                // arguments through: `x[order(-x)]`, `x[between(x, -1, 1)]`.
                // Only assume this for unqualified calls and known namespaces.
                let skips_negative_arguments = matches!(
                    (
                        get_function_name(function.clone()).as_str(),
                        get_function_namespace_prefix(function).as_deref()
                    ),
                    ("which" | "order", None | Some("base::"))
                        | ("between", None | Some("dplyr::" | "data.table::"))
                );
                // Literal-only `c()` calls, such as `x[c(-1, -2)]`, cannot
                // contain runtime-dependent negative indices either.
                if skips_negative_arguments || is_literal_c_call(&call)? {
                    preorder.skip_subtree();
                }
            }
            _ => {}
        }
    }
    Ok(false)
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
        AnyRExpression::RCall(call) => is_literal_c_call(&call),
        AnyRExpression::RUnaryExpression(unary) => Ok(matches!(
            unary.operator()?.kind(),
            RSyntaxKind::PLUS | RSyntaxKind::MINUS
        ) && is_literal(unary.argument()?)?),
        _ => Ok(false),
    }
}
