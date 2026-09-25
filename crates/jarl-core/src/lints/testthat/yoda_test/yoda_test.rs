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
/// Checks for literals supplied as `object` in `expect_equal()`,
/// `expect_identical()`, and `expect_setequal()`. Also reports comparisons
/// where both arguments are literals.
///
/// ## Why is this bad?
///
/// Testthat expectations take the actual result first and the expected value
/// second. Putting the expected value first is called a "Yoda test". Following
/// the usual order makes tests easier to read and failure messages clearer.
///
/// Comparisons of two literals, such as `expect_equal(1, 1)`, do not test the
/// result of any application code. These require a meaningful test instead
/// and cannot be fixed automatically.
///
/// This rule is **disabled by default**. Select it either with the rule name
/// `"yoda_test"` or with the rule group `"TESTTHAT"`.
///
/// ## Example
///
/// ```r
/// expect_equal(2, length(x))
/// expect_identical("a", get_name(x))
/// expect_setequal(1L, unique(x))
/// ```
///
/// Use instead:
/// ```r
/// expect_equal(length(x), 2)
/// expect_identical(get_name(x), "a")
/// expect_setequal(unique(x), 1L)
/// ```
/// <!-- docs: end -->
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
                Rule::TestthatYodaTest,
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
            Rule::TestthatYodaTest,
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
