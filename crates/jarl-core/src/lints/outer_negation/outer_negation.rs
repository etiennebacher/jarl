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
    // We don't want to report calls like !any(x), just any(x)
    if let Some(parent) = ast.syntax().parent() {
        if let Some(prev_sibling) = ast.syntax().prev_sibling() {
            if parent.kind() == RSyntaxKind::R_UNARY_EXPRESSION
                && prev_sibling.kind() == RSyntaxKind::BANG
            {
                return Ok(None);
            }
        }
    };

    let function = ast.function()?;
    let function_name = function.to_trimmed_string();

    if function_name != "all" && function_name != "any" {
        return Ok(None);
    };

    let args = ast.arguments()?.items();
    let arg = get_arg_by_position(&args, 1);
    if arg.is_none() {
        return Ok(None);
    }

    let n_args = args
        .into_iter()
        .map(|x| x.unwrap())
        .collect::<Vec<_>>()
        .len();

    if n_args > 1 {
        return Ok(None);
    }

    let arg = arg.unwrap().value().unwrap();
    if arg.syntax().kind() != RSyntaxKind::R_UNARY_EXPRESSION {
        return Ok(None);
    };

    // It looks like the first (and only) child of R_UNARY_EXPRESSION is what
    // comes after "!". So we don't need to check that this is indeed using the
    // BANG operator because it's the only R_UNARY_EXPRESSION available.

    let content = if let Some(expr) = arg.syntax().first_child() {
        // We don't want to report consecutive unary expressions, e.g. any(!!x),
        // because they could be special syntax.
        if expr.kind() == RSyntaxKind::R_UNARY_EXPRESSION {
            return Ok(None);
        };
        expr.text_trimmed().to_string()
    } else {
        "".to_string()
    };

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
