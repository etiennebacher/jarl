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
/// `expect_setequal()`. Also reports comparisons where both values are literals.
///
/// ## Why is this bad?
///
/// Putting the expected value before the actual result, as in `1 == x`, is
/// called a "Yoda condition". Writing the actual result first makes comparisons
/// easier to read and testthat failure messages clearer.
///
/// Comparisons of two literals, such as `1 == 1` or `expect_equal(1, 1)`, do not
/// test the result of application code and cannot be fixed automatically.
///
/// This rule is **disabled by default**. Select it either with the rule name
/// `"yoda_condition"` or with the rule group `"READ"`.
///
/// Automatic fixes require `--unsafe-fixes`: swapping values can change custom
/// comparison methods, expectation return values, or tolerance-based results.
///
/// ## Example
///
/// ```r
/// 1 == x
/// 10 < length(x)
/// expect_equal(2, length(x))
/// ```
///
/// Use instead:
/// ```r
/// x == 1
/// length(x) > 10
/// expect_equal(length(x), 2)
/// ```
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
    if !is_literal(&left)? {
        return Ok(None);
    }
    let right = ast.right()?;
    let range = ast.syntax().text_trimmed_range();

    // Comparisons of two literals.
    if is_literal(&right)? {
        return Ok(Some(Diagnostic::new(
            ViolationData::new(
                Rule::YodaCondition,
                "Comparing two literals does not test an actual result.".to_string(),
                Some("Compare an actual result with the expected literal instead.".to_string()),
            ),
            range,
            Fix::empty(),
        )));
    }

    // A right operand can contain a low-precedence prefix or an open-ended
    // body. Keep it grouped when moving it left, e.g. `1 == !x` -> `(!x) == 1`.
    let left_text = left.to_trimmed_string();
    let right_text = if needs_comparison_parentheses(&right)? {
        format!("({})", right.to_trimmed_text())
    } else {
        right.to_trimmed_string()
    };
    let suggestion = format!("Use `{right_text} {replacement_operator} {left_text}` instead.");

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::YodaCondition,
            "The actual result should come before the expected literal.".to_string(),
            Some(suggestion),
        ),
        range,
        Fix::from_edits(
            vec![
                Edit::replacement(left.syntax().text_trimmed_range(), right_text),
                Edit::replacement(
                    operator.text_trimmed_range(),
                    replacement_operator.to_string(),
                ),
                Edit::replacement(right.syntax().text_trimmed_range(), left_text),
            ],
            node_contains_comments(ast.syntax()),
        ),
    )))
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
    if !is_literal(&object_value)? {
        return Ok(None);
    }

    let expected = unwrap_or_return_none!(get_arg(ast, FORMALS_EXPECT, "expected"));
    let expected_value = unwrap_or_return_none!(expected.value());

    let range = ast.syntax().text_trimmed_range();
    if is_literal(&expected_value)? {
        return Ok(Some(Diagnostic::new(
            ViolationData::new(
                Rule::YodaCondition,
                "Comparing two literals does not test an actual result.".to_string(),
                Some("Compare an actual result with the expected literal instead.".to_string()),
            ),
            range,
            Fix::empty(),
        )));
    }

    // Keep named arguments and custom labels for manual review. With two
    // unnamed arguments, only their values need to move; retain all other text.
    let fix = if args.len() == 2
        && object.name_clause().is_none()
        && expected.name_clause().is_none()
        && !node_contains_comments(ast.syntax())
    {
        Fix::from_edits(
            vec![
                Edit::replacement(
                    object_value.syntax().text_trimmed_range(),
                    expected_value.to_trimmed_string(),
                ),
                Edit::replacement(
                    expected_value.syntax().text_trimmed_range(),
                    object_value.to_trimmed_string(),
                ),
            ],
            false,
        )
    } else {
        Fix::empty()
    };

    let object_text = object_value.to_trimmed_text();
    let expected_text = expected_value.to_trimmed_text();
    let function = ast.function()?.to_trimmed_text();
    let suggestion = format!("Use `{function}({expected_text}, {object_text})` instead.");

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::YodaCondition,
            "The actual result should be supplied as `object`, and the expected literal as `expected`."
                .to_string(),
            Some(suggestion),
        ),
        range,
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
        return Ok(operator.kind() == RSyntaxKind::PIPE
            || (operator.kind() == RSyntaxKind::SPECIAL
                && matches!(operator.text_trimmed(), "%>%" | "%!>%" | "%T>%" | "%<>%")));
    }
    Ok(false)
}
