use crate::diagnostic::*;
use crate::utils::{get_function_name, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstNodeList};

/// ## What it does
///
/// Checks for usage of `if (is.null(x)) y else x` or
/// `if (!is.null(x)) x else y` and recommends using `x %||% y` instead.
///
/// ## Why is this bad?
///
/// Using the coalesce operator `%||%` is more concise and readable than
/// an if-else statement checking for null.
///
/// This rule is only enabled if the project uses R >= 4.4.0, since `%||%` was
/// introduced in this version.
///
/// This rule contains some automatic fixes, but only for cases where the
/// branches are on a single line. For instance,
/// ```r,ignore
/// if (is.null(x)) {
///   y
/// } else {
///   x
/// }
/// ```
/// would be simplified to `x %||% y`, but
/// ```r,ignore
/// if (is.null(x)) {
///   y <- 1
///   y
/// } else {
///   x
/// }
/// ```
/// wouldn't.
///
/// ## Example
///
/// ```r
/// x <- 1
/// y <- 2
///
/// if (is.null(x)) y else x
///
/// if (!is.null(x)) {
///   x
/// } else {
///   y
/// }
/// ```
///
/// Use instead:
/// ```r
/// x <- 1
/// y <- 2
///
/// x %||% y # (in both cases)
/// ```
///
/// ## Reference
///
/// See `?Control`
pub fn coalesce(ast: &RIfStatement) -> anyhow::Result<Option<Diagnostic>> {
    let condition = ast.condition()?;
    let consequence = ast.consequence()?;
    let alternative = if let Some(else_clause) = ast.else_clause() {
        else_clause.alternative()?
    } else {
        return Ok(None);
    };

    let mut msg = "".to_string();
    let mut fix_content = "".to_string();
    let mut skip_fix = false;

    // Check if consequence or alternative have multiple expressions
    let consequence_has_multiple = has_multiple_expressions(&consequence);
    let alternative_has_multiple = has_multiple_expressions(&alternative);

    // If either has multiple expressions, we'll report but not fix
    if consequence_has_multiple || alternative_has_multiple {
        skip_fix = true;
    }

    // Case 1:
    // if (is.null(x)) y else x  => x %||% y
    if let Some(condition) = condition.as_r_call() {
        let function = condition.function()?;
        let fn_name = get_function_name(function);
        if fn_name != "is.null" {
            return Ok(None);
        }

        let fn_body = condition
            .arguments()?
            .items()
            .into_iter()
            .filter_map(Result::ok)
            .filter_map(|x| x.value())
            .collect::<Vec<AnyRExpression>>();

        if fn_body.len() != 1 {
            return Ok(None);
        }

        let fn_body = fn_body.first().unwrap();
        let alternative_str = extract_single_expression(&alternative);
        let consequence_str = extract_single_expression(&consequence);

        let inside_null_same_as_alternative = fn_body.to_trimmed_string() == alternative_str;

        if !inside_null_same_as_alternative {
            return Ok(None);
        }

        msg = "`if (is.null(x)) y else x` can be simplified.".to_string();

        if !skip_fix {
            fix_content = format!("{} %||% {}", fn_body.to_trimmed_string(), consequence_str);
        }
    }

    // Case 2:
    // if (!is.null(x)) x else y  => x %||% y
    if let Some(condition) = condition.as_r_unary_expression() {
        let operator = condition.operator()?;
        if operator.text_trimmed() != "!" {
            return Ok(None);
        }

        let function = condition.argument()?;
        let Some(call) = function.as_r_call() else {
            return Ok(None);
        };
        let function = call.function()?;

        let fn_name = get_function_name(function);
        if fn_name != "is.null" {
            return Ok(None);
        }

        let fn_body = call
            .arguments()?
            .items()
            .into_iter()
            .filter_map(Result::ok)
            .filter_map(|x| x.value())
            .collect::<Vec<AnyRExpression>>();

        if fn_body.len() != 1 {
            return Ok(None);
        }

        let fn_body = fn_body.first().unwrap();
        let consequence_str = extract_single_expression(&consequence);
        let alternative_str = extract_single_expression(&alternative);

        let inside_null_same_as_consequence = fn_body.to_trimmed_string() == consequence_str;

        if !inside_null_same_as_consequence {
            return Ok(None);
        }

        msg = "`if (!is.null(x)) x else y` can be simplified.".to_string();

        if !skip_fix {
            fix_content = format!("{} %||% {}", fn_body.to_trimmed_string(), alternative_str);
        }
    }

    if msg.is_empty() {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "coalesce".to_string(),
            msg,
            Some("Use `x %||% y` instead.".to_string()),
        ),
        range,
        Fix {
            content: fix_content.clone(),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()) || skip_fix,
        },
    );

    Ok(Some(diagnostic))
}

// Check if an expression has multiple statements
fn has_multiple_expressions(input: &AnyRExpression) -> bool {
    if let Some(braced) = input.as_r_braced_expressions() {
        let expressions = braced.expressions();
        expressions.len() > 1
    } else {
        false
    }
}

// Extract single expression from braced expressions, or return the expression as-is
fn extract_single_expression(input: &AnyRExpression) -> String {
    if let Some(braced) = input.as_r_braced_expressions() {
        let expressions: Vec<_> = braced.expressions().into_iter().collect();
        if expressions.len() == 1 {
            // Single expression in braces, extract it
            if let Some(expr) = expressions.first() {
                return expr.to_trimmed_string();
            }
        }
    }

    input.to_trimmed_string()
}
