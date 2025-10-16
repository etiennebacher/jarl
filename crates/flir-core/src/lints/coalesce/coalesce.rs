use crate::diagnostic::*;
use crate::utils::{get_function_name, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for usage of `grep(..., value = TRUE)` and recommends using
/// `grepv()` instead (only if the R version used in the project is >= 4.5).
///
/// ## Why is this bad?
///
/// Starting from R 4.5, there is a function `grepv()` that is identical to
/// `grep()` except that it uses `value = TRUE` by default.
///
/// Using `grepv(...)` is therefore more readable than `grep(...)`.
///
/// ## Example
///
/// ```r
/// x <- c("hello", "hi", "howdie")
/// grep("i", x, value = TRUE)
/// ```
///
/// Use instead:
/// ```r
/// x <- c("hello", "hi", "howdie")
/// grepv("i", x)
/// ```
///
/// ## References
///
/// See `?grepv`
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
            .map(|x| x)
            .collect::<Vec<AnyRExpression>>();

        if fn_body.len() != 1 {
            return Ok(None);
        }

        let fn_body = fn_body.first().unwrap();
        let alternative = remove_curly_braces(&alternative);
        let consequence = remove_curly_braces(&consequence);

        let inside_null_same_as_alternative = fn_body.to_trimmed_string() == alternative;

        if !inside_null_same_as_alternative {
            return Ok(None);
        }

        msg = "Use `x %||% y` instead of `if (is.null(x)) y else x`.".to_string();
        fix_content = format!("{} %||% {}", fn_body.to_trimmed_string(), consequence);
    }

    // Case 2:
    // if (!is.null(x)) x else y  => x %||% y
    if let Some(condition) = condition.as_r_unary_expression() {
        let operator = condition.operator()?;
        if operator.text_trimmed() != "!" {
            return Ok(None);
        }

        let function = condition.argument()?;
        let call = function.as_r_call();
        if call.is_none() {
            return Ok(None);
        }
        let call = call.unwrap();
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
            .map(|x| x)
            .collect::<Vec<AnyRExpression>>();

        if fn_body.len() != 1 {
            return Ok(None);
        }

        let fn_body = fn_body.first().unwrap();
        let consequence = remove_curly_braces(&consequence);
        let alternative = remove_curly_braces(&alternative);

        let inside_null_same_as_consequence = fn_body.to_trimmed_string() == consequence;

        if !inside_null_same_as_consequence {
            return Ok(None);
        }

        msg = "Use `x %||% y` instead of `if (!is.null(x)) x else y`.".to_string();
        fix_content = format!("{} %||% {}", fn_body.to_trimmed_string(), alternative);
    }

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new("coalesce".to_string(), msg),
        range,
        Fix {
            content: fix_content,
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}

fn remove_curly_braces(input: &AnyRExpression) -> String {
    if let Some(input) = input.as_r_braced_expressions() {
        let expressions = input.expressions().into_iter();
        if expressions.len() == 1 {
            return input.to_trimmed_string();
        }

        expressions
            .map(|x| x.to_trimmed_string())
            .collect::<Vec<String>>()
            .join("\n")
    } else {
        input.to_trimmed_string()
    }
}
