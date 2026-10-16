use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{Formals, get_arg, get_function_namespace_prefix, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

// All three expectations share these first two formals.
const FORMALS_EXPECT: Formals = &["object", "expected"];

/// <!-- docs: start -->
/// Version added: 0.7.0
///
/// ## What it does
///
/// Checks for literals on the left of `==`, `!=`, `<`, `<=`, `>`, and `>=`,
/// or supplied as `object` in `expect_equal()`, `expect_identical()`, and
/// `expect_setequal()`.
///
/// ## Why is this bad?
///
/// Putting the expected value before the actual result, as in `1 == x`, is
/// called a "Yoda condition". Writing the actual result first makes comparisons
/// easier to read.
///
/// This rule has an unsafe automatic fix because swapping values can change
/// results, for instance due to S3 method dispatch.
///
/// ## Example
///
/// ```r
/// 1 == x
/// 10 < length(x)
/// expect_equal(2, x + y)
/// ```
///
/// Use instead:
/// ```r
/// x == 1
/// length(x) > 10
/// expect_equal(x + y, 2)
/// ```
///
/// ## References
///
/// * https://en.wikipedia.org/wiki/Yoda_conditions
/// <!-- docs: end -->
/// Checks binary comparisons whose left operand is a literal.
pub fn yoda_condition(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let operator = ast.operator()?;
    let replacement_operator = match operator.kind() {
        RSyntaxKind::EQUAL2 => "==",
        RSyntaxKind::NOT_EQUAL => "!=",
        RSyntaxKind::LESS_THAN => ">",
        RSyntaxKind::LESS_THAN_OR_EQUAL_TO => ">=",
        RSyntaxKind::GREATER_THAN => "<",
        RSyntaxKind::GREATER_THAN_OR_EQUAL_TO => "<=",
        _ => return Ok(None),
    };

    let left = ast.left()?;
    let right = ast.right()?;
    if !is_yoda(&left, &right)? {
        return Ok(None);
    }

    // A right operand can contain a low-precedence prefix or an open-ended
    // body. Keep it grouped when moving it left, e.g. `1 == !x` -> `(!x) == 1`.
    let (left_edit, right_edit) = swap_edits(&left, &right, needs_comparison_parentheses(&right)?);

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::YodaCondition,
            "Putting the literal first in the comparison is less readable.".to_string(),
            Some("Put the actual result on the left and the literal on the right.".to_string()),
        ),
        ast.syntax().text_trimmed_range(),
        Fix::from_edits(
            vec![
                left_edit,
                Edit::replacement(
                    operator.text_trimmed_range(),
                    replacement_operator.to_string(),
                ),
                right_edit,
            ],
            node_contains_comments(ast.syntax()),
        ),
    )))
}

/// A literal compared with something that is not itself a literal. Two
/// literals have no actual result to reorder.
fn is_yoda(literal: &AnyRExpression, other: &AnyRExpression) -> anyhow::Result<bool> {
    Ok(is_literal(literal)? && !is_literal(other)?)
}

/// Edits exchanging the text of `a` and `b`, wrapping `b` in parentheses when
/// it moves into `a`'s position and `parenthesize_b` is set.
fn swap_edits(a: &AnyRExpression, b: &AnyRExpression, parenthesize_b: bool) -> (Edit, Edit) {
    let b_text = if parenthesize_b {
        format!("({})", b.to_trimmed_string())
    } else {
        b.to_trimmed_string()
    };
    (
        Edit::replacement(a.syntax().text_trimmed_range(), b_text),
        Edit::replacement(b.syntax().text_trimmed_range(), a.to_trimmed_string()),
    )
}

fn needs_comparison_parentheses(expr: &AnyRExpression) -> anyhow::Result<bool> {
    match expr {
        AnyRExpression::RBinaryExpression(binary) => Ok(
            OperatorPrecedence::try_from_binary_operator(binary.operator()?.kind())
                .is_none_or(|precedence| precedence <= OperatorPrecedence::Relational)
                // Even arithmetic can end with a prefix that absorbs a comparison:
                // `1 == x + !y` must become `(x + !y) == 1`.
                || needs_comparison_parentheses(&binary.right()?)?,
        ),
        AnyRExpression::RUnaryExpression(unary) => {
            Ok(!matches!(
                unary.operator()?.kind(),
                RSyntaxKind::PLUS | RSyntaxKind::MINUS
            ) || needs_comparison_parentheses(&unary.argument()?)?)
        }
        AnyRExpression::RIfStatement(_)
        | AnyRExpression::RFunctionDefinition(_)
        | AnyRExpression::RForStatement(_)
        | AnyRExpression::RWhileStatement(_)
        | AnyRExpression::RRepeatStatement(_) => Ok(true),
        _ => Ok(false),
    }
}

/// Checks testthat expectations whose `object` argument is a literal.
pub fn yoda_test(ast: &RCall, fn_name: &str) -> anyhow::Result<Option<Diagnostic>> {
    if !matches!(
        fn_name,
        "expect_equal" | "expect_identical" | "expect_setequal"
    ) {
        return Ok(None);
    }
    if let Some(namespace) = get_function_namespace_prefix(ast.function()?)
        && namespace != "testthat::"
    {
        return Ok(None);
    }
    if receives_pipe_input(ast)? {
        return Ok(None);
    }

    let args = ast.arguments()?.items();
    // Forwarded arguments can change which positional argument binds to `object`.
    for arg in args.iter() {
        if matches!(arg?.value(), Some(AnyRExpression::RDots(_))) {
            return Ok(None);
        }
    }

    let object = unwrap_or_return_none!(get_arg(ast, FORMALS_EXPECT, "object"));
    let object_value = unwrap_or_return_none!(object.value());
    let expected = unwrap_or_return_none!(get_arg(ast, FORMALS_EXPECT, "expected"));
    let expected_value = unwrap_or_return_none!(expected.value());
    if !is_yoda(&object_value, &expected_value)? {
        return Ok(None);
    }

    // Keep named arguments and custom labels for manual review. With two
    // unnamed arguments, only their values need to move; retain all other text.
    let fix = if args.len() == 2
        && object.name_clause().is_none()
        && expected.name_clause().is_none()
        && !node_contains_comments(ast.syntax())
    {
        let (object_edit, expected_edit) = swap_edits(&object_value, &expected_value, false);
        Fix::from_edits(vec![object_edit, expected_edit], false)
    } else {
        Fix::empty()
    };

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::YodaCondition,
            "Using a literal in the `object` argument is less readable and gives worse error messages."
                .to_string(),
            Some("Supply the actual result as `object` and the literal as `expected`.".to_string()),
        ),
        ast.syntax().text_trimmed_range(),
        fix,
    )))
}

fn is_literal(expr: &AnyRExpression) -> anyhow::Result<bool> {
    match expr {
        AnyRExpression::AnyRValue(AnyRValue::RStringValue(_)) => Ok(true),
        AnyRExpression::RParenthesizedExpression(paren) => is_literal(&paren.body()?),
        // Match lintr's simple arithmetic literals, including complex numbers.
        AnyRExpression::RBinaryExpression(binary) => Ok(matches!(
            binary.operator()?.kind(),
            RSyntaxKind::PLUS | RSyntaxKind::MINUS
        ) && is_numeric_literal(&binary.left()?)?
            && is_numeric_literal(&binary.right()?)?),
        _ => is_numeric_literal(expr),
    }
}

fn is_numeric_literal(expr: &AnyRExpression) -> anyhow::Result<bool> {
    match expr {
        AnyRExpression::AnyRValue(
            AnyRValue::RIntegerValue(_) | AnyRValue::RDoubleValue(_) | AnyRValue::RComplexValue(_),
        )
        | AnyRExpression::RTrueExpression(_)
        | AnyRExpression::RFalseExpression(_)
        | AnyRExpression::RNaExpression(_)
        | AnyRExpression::RInfExpression(_)
        | AnyRExpression::RNanExpression(_) => Ok(true),
        AnyRExpression::RParenthesizedExpression(paren) => is_numeric_literal(&paren.body()?),
        AnyRExpression::RUnaryExpression(unary) => Ok(matches!(
            unary.operator()?.kind(),
            RSyntaxKind::PLUS | RSyntaxKind::MINUS
        ) && is_numeric_literal(&unary.argument()?)?),
        _ => Ok(false),
    }
}

/// Whether the call is the right-hand side of a pipe that supplies its first
/// argument. A magrittr pipe with an explicit `.` argument does not.
fn receives_pipe_input(ast: &RCall) -> anyhow::Result<bool> {
    let mut node = ast.syntax().clone();
    while let Some(parent) = node.parent() {
        if RParenthesizedExpression::can_cast(parent.kind()) {
            node = parent;
            continue;
        }
        let Some(binary) = RBinaryExpression::cast(parent) else {
            return Ok(false);
        };
        if binary.right()?.syntax() != &node {
            return Ok(false);
        }
        let operator = binary.operator()?;
        if operator.kind() == RSyntaxKind::PIPE {
            return Ok(true);
        }
        if operator.kind() != RSyntaxKind::SPECIAL
            || !matches!(operator.text_trimmed(), "%>%" | "%!>%" | "%T>%" | "%<>%")
        {
            return Ok(false);
        }
        for arg in ast.arguments()?.items().iter() {
            if let Some(AnyRExpression::RIdentifier(id)) = arg?.value()
                && id.syntax().text_trimmed() == "."
            {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    Ok(false)
}
