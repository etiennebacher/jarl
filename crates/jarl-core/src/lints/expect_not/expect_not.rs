use crate::diagnostic::*;
use crate::utils::{get_arg_by_name_then_position, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

/// ## What it does
///
/// Checks for usage of `expect_true(!x)` and `expect_false(!x)` in tests.
///
/// ## Why is this bad
///
/// Using `expect_false(x)` is clearer and more direct than `expect_true(!x)`,
/// and vice versa.
///
/// This rule is **disabled by default**. Select it either with the rule name
/// `"expect_not"` or with the rule group `"TESTTHAT"`.
///
/// ## Example
///
/// ```r
/// expect_true(!x)
/// expect_false(!foo(x))
/// expect_true(!(x && y))
///
/// # rlang "!!!" operator is left unmodified
/// expect_true(!!!x)
/// ```
///
/// Use instead:
/// ```r
/// expect_false(x)
/// expect_true(foo(x))
/// expect_false(x && y)
///
/// # rlang "!!!" operator is left unmodified
/// expect_true(!!!x)
/// ```
pub fn expect_not(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let function_name = function.to_trimmed_text();

    // Only check expect_true and expect_false
    if function_name != "expect_true" && function_name != "expect_false" {
        return Ok(None);
    }

    let args = ast.arguments()?.items();

    // Get the first argument (object)
    let Some(object) = get_arg_by_name_then_position(&args, "object", 1) else {
        return Ok(None);
    };

    // Skip if there are multiple arguments (e.g., expect_true(!x, label = "test"))
    // Only lint when there's exactly one argument
    if args.iter().count() > 1 {
        return Ok(None);
    }

    let Some(object_value) = object.value() else {
        return Ok(None);
    };

    // Check if it's a unary expression (negation)
    let Some(unary_expr) = object_value.as_r_unary_expression() else {
        return Ok(None);
    };
    let Ok(operator) = unary_expr.operator() else {
        return Ok(None);
    };
    if operator.kind() != RSyntaxKind::BANG {
        return Ok(None);
    }

    // Get the argument after the negation
    let Ok(argument) = unary_expr.argument() else {
        return Ok(None);
    };

    // Check for rlang bang-bang (!!, !!!) - we should skip these
    if let Some(inner_unary) = argument.as_r_unary_expression()
        && let Ok(inner_op) = inner_unary.operator()
        && inner_op.kind() == RSyntaxKind::BANG
    {
        // This is !! or !!!, skip it
        return Ok(None);
    }

    // Strip outer parentheses if present: !(x && y) -> x && y
    let inner_text = if let Some(paren_expr) = argument.as_r_parenthesized_expression() {
        if let Ok(body) = paren_expr.body() {
            body.to_trimmed_text()
        } else {
            argument.to_trimmed_text()
        }
    } else {
        argument.to_trimmed_text()
    };

    // Determine the replacement function
    let (current_fn, replacement_fn) = if function_name == "expect_true" {
        ("expect_true", "expect_false")
    } else {
        ("expect_false", "expect_true")
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "expect_not".to_string(),
            format!(
                "`{}(!x)` is not as clear as `{}(x)`.",
                current_fn, replacement_fn
            ),
            Some(format!("Use `{}(x)` instead.", replacement_fn)),
        ),
        range,
        Fix {
            content: format!("{}({})", replacement_fn, inner_text),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
