use crate::diagnostic::*;
use crate::utils::{get_arg_by_position, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for usage of `all(!x)` or `any(!x)`.
///
/// ## Why is this bad?
///
/// Those two patterns may be hard to read and understand, especially when the
/// expression after `!` is lengthy. Using `!any(x)` instead of `all(!x)` and
/// `!all(x)` instead of `any(!x)` may be more readable.
///
/// In addition, using the `!` operator outside the function call is more
/// efficient since it only has to invert one value instead of all values inside
/// the function call.
///
/// ## Example
///
/// ```r
/// any(!x)
/// all(!x)
/// ```
///
/// Use instead:
/// ```r
/// !all(x)
/// !any(x)
/// ```
pub fn outer_negation(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    // We don't want to report calls like `!any(x)`, just `any(x)`
    if let Some(parent) = ast.syntax().parent()
        && parent.kind() == RSyntaxKind::R_UNARY_EXPRESSION
        && let Some(prev_sibling) = ast.syntax().prev_sibling()
        && prev_sibling.kind() == RSyntaxKind::BANG
    {
        return Ok(None);
    }

    let function = ast.function()?;
    let function_name = function.to_trimmed_string();

    if function_name != "all" && function_name != "any" {
        return Ok(None);
    };

    let args = ast.arguments()?.items();

    // Only check calls with exactly one argument
    let Some(first_arg) = get_arg_by_position(&args, 1) else {
        return Ok(None);
    };

    // Ensure there's not more than one argument (e.g. skip `any(!x, y)`).
    if get_arg_by_position(&args, 2).is_some() {
        return Ok(None);
    }

    let Some(arg_value) = first_arg.value() else {
        return Ok(None);
    };
    // Check if the argument is a unary expression (negation)
    if arg_value.syntax().kind() != RSyntaxKind::R_UNARY_EXPRESSION {
        return Ok(None);
    }

    // Get the expression after the negation operator
    // Skip the BANG token to get the actual expression
    let negated_expr = arg_value
        .syntax()
        .children()
        .find(|child| child.kind() != RSyntaxKind::BANG);

    let Some(expr) = negated_expr else {
        return Ok(None);
    };

    // It looks like the first (and only) child of R_UNARY_EXPRESSION is what
    // comes after "!". So we don't need to check that this is indeed using the
    // BANG operator because it's the only R_UNARY_EXPRESSION available.

    // Don't report consecutive unary expressions (e.g., `any(!!x)`)
    if expr.kind() == RSyntaxKind::R_UNARY_EXPRESSION {
        return Ok(None);
    }

    let content = expr.text_trimmed().to_string();

    let (replacement_function, msg, suggestion) = match function_name.as_str() {
        "any" => (
            "all",
            "`any(!x)` may be hard to read.",
            "Use `!all(x)` instead.",
        ),
        "all" => (
            "any",
            "`all(!x)` may be hard to read.",
            "Use `!any(x)` instead.",
        ),
        _ => unreachable!(),
    };

    let fix = format!("!{}({})", replacement_function, content);
    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "outer_negation".to_string(),
            msg.to_string(),
            Some(suggestion.to_string()),
        ),
        range,
        Fix {
            content: fix,
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
