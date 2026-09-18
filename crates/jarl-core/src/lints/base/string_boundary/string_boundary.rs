use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{Formals, get_arg, get_function_name, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};
use oak_core::syntax_ext::RStringValueExt;

/// Version added: 0.3.0
///
/// ## What it does
///
/// Checks for `substr()` and `substring()` calls that can be replaced with
/// `startsWith()` or `endsWith()`.
/// Only comparisons to non-empty string literals with matching substring
/// boundaries are reported. Ordinary strings containing escapes are skipped.
///
/// ## Why is this bad?
///
/// Using `startsWith()` and `endsWith()` is both more readable and more efficient
/// than extracting substrings and comparing them.
///
/// This rule has an unsafe fix because the replacement can drop names and other
/// attributes, no longer coerces non-character inputs, and may evaluate repeated
/// expressions fewer times.
///
/// ## Example
///
/// ```r
/// substr(x, 1L, 3L) == "abc"
/// substring(x, nchar(x) - 2L, nchar(x)) == "xyz"
/// ```
/// Use instead:
/// ```r
/// startsWith(x, "abc")
/// endsWith(x, "xyz")
/// ```
///
/// ## References
///
/// See `?startsWith` and `?substr`
pub fn string_boundary(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let operator = operator?;
    let op_kind = operator.kind();

    // Only check == and != operators
    if op_kind != RSyntaxKind::EQUAL2 && op_kind != RSyntaxKind::NOT_EQUAL {
        return Ok(None);
    }

    let left = left?;
    let right = right?;

    // Check if either side is a function call to substr or substring
    let (call, string_expr) = if let AnyRExpression::RCall(c) = &left {
        (c, &right)
    } else if let AnyRExpression::RCall(c) = &right {
        (c, &left)
    } else {
        return Ok(None);
    };

    // Check if it's substr or substring
    let function = call.function()?;
    let func_name = get_function_name(function);

    if func_name != "substr" && func_name != "substring" {
        return Ok(None);
    }

    if call.arguments()?.items().len() != 3 {
        return Ok(None);
    }

    let formals: Formals = if func_name == "substr" {
        &["x", "start", "stop"]
    } else {
        &["text", "first", "last"]
    };
    let x_arg =
        unwrap_or_return_none!(get_arg(call, formals, formals[0]).and_then(|arg| arg.value()));
    let start_arg =
        unwrap_or_return_none!(get_arg(call, formals, formals[1]).and_then(|arg| arg.value()));
    let end_arg =
        unwrap_or_return_none!(get_arg(call, formals, formals[2]).and_then(|arg| arg.value()));
    let width = unwrap_or_return_none!(literal_string_length(string_expr));

    // Get the string being compared
    let string_text = string_expr.syntax().text_trimmed();
    let x_text = x_arg.syntax().text_trimmed();

    if literal_integer(&start_arg) == Some(1) && literal_integer(&end_arg) == Some(width) {
        let range = ast.syntax().text_trimmed_range();

        // Build the replacement: startsWith(x, "string") or !startsWith(x, "string")
        let replacement = if op_kind == RSyntaxKind::NOT_EQUAL {
            format!("!startsWith({}, {})", x_text, string_text)
        } else {
            format!("startsWith({}, {})", x_text, string_text)
        };

        let diagnostic = Diagnostic::new(
            ViolationData::new(
                Rule::StringBoundary,
                format!(
                    "Using `{func_name}()` to detect an initial substring is hard to read and inefficient."
                ),
                Some("Use `startsWith()` instead.".to_string()),
            ),
            range,
            Fix::new(range, replacement, node_contains_comments(ast.syntax())),
        );
        return Ok(Some(diagnostic));
    }

    if is_nchar_of_same_expr(&end_arg, &x_arg) && is_suffix_start(&start_arg, &x_arg, width) {
        let range = ast.syntax().text_trimmed_range();

        // Build the replacement: endsWith(x, "string") or !endsWith(x, "string")
        let replacement = if op_kind == RSyntaxKind::NOT_EQUAL {
            format!("!endsWith({}, {})", x_text, string_text)
        } else {
            format!("endsWith({}, {})", x_text, string_text)
        };

        let diagnostic = Diagnostic::new(
            ViolationData::new(
                Rule::StringBoundary,
                format!(
                    "Using `{func_name}()` to detect a terminal substring is hard to read and inefficient."
                ),
                Some("Use `endsWith()` instead.".to_string()),
            ),
            range,
            Fix::new(range, replacement, node_contains_comments(ast.syntax())),
        );
        return Ok(Some(diagnostic));
    }

    Ok(None)
}

fn literal_string_length(expr: &AnyRExpression) -> Option<usize> {
    let string = expr.as_any_r_value()?.as_r_string_value()?;
    let content = string.string_text()?;
    let open = string.open_token().ok()?;
    let is_raw = open.text_trimmed().starts_with(['r', 'R']);

    // Token contents retain escapes, whose source length differs from R's nchar().
    if content.is_empty() || (!is_raw && content.contains('\\')) {
        return None;
    }
    Some(content.chars().count())
}

fn literal_integer(expr: &AnyRExpression) -> Option<usize> {
    let token = match expr.as_any_r_value()? {
        AnyRValue::RIntegerValue(value) => value.value_token().ok()?,
        AnyRValue::RDoubleValue(value) => value.value_token().ok()?,
        _ => return None,
    };
    let value = token
        .text_trimmed()
        .trim_end_matches('L')
        .parse::<f64>()
        .ok()?;
    // Substring indices are coerced to R integers; reject fractions and overflow.
    (value >= 0.0 && value <= f64::from(i32::MAX) && value.fract() == 0.0).then_some(value as usize)
}

fn is_suffix_start(start: &AnyRExpression, x: &AnyRExpression, width: usize) -> bool {
    if width == 1 && is_nchar_of_same_expr(start, x) {
        return true;
    }
    let Some(binary) = start.as_r_binary_expression() else {
        return false;
    };
    binary
        .operator()
        .is_ok_and(|op| op.kind() == RSyntaxKind::MINUS)
        && binary
            .left()
            .is_ok_and(|left| is_nchar_of_same_expr(&left, x))
        && binary
            .right()
            .is_ok_and(|right| literal_integer(&right) == width.checked_sub(1))
}

/// Check if end_expr is nchar(x_expr) where x_expr matches the first argument
fn is_nchar_of_same_expr(end_expr: &AnyRExpression, x_expr: &AnyRExpression) -> bool {
    // Check if end_expr is a function call
    let call = match end_expr {
        AnyRExpression::RCall(c) => c,
        _ => return false,
    };

    // Check if it's nchar()
    let function = match call.function() {
        Ok(f) => f,
        _ => return false,
    };

    let func_name = get_function_name(function);
    if func_name != "nchar" {
        return false;
    }

    // Get the argument to nchar()
    let arguments = match call.arguments() {
        Ok(a) => a,
        _ => return false,
    };

    if arguments.items().len() != 1 {
        return false;
    }

    // Get the expression from the first argument
    let nchar_arg = match get_arg(call, &["x"], "x").and_then(|arg| arg.value()) {
        Some(v) => v,
        None => return false,
    };

    // Compare if nchar's argument matches x_expr syntactically
    expressions_match(&nchar_arg, x_expr)
}

/// Check if two expressions are syntactically identical
fn expressions_match(expr1: &AnyRExpression, expr2: &AnyRExpression) -> bool {
    expr1.syntax().text_trimmed() == expr2.syntax().text_trimmed()
}
