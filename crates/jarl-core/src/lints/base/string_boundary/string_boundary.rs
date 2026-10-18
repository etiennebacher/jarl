use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{Formals, get_arg, get_function_name, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};
use oak_semantic::effects::CallContext;

/// <!-- docs: start -->
/// Version added: 0.3.0
///
/// ## What it does
///
/// Checks for `substr()` and `substring()` calls that can be replaced with
/// `startsWith()` or `endsWith()`.
/// Only comparisons to non-empty string literals with matching substring
/// boundaries are reported. Strings containing carriage returns, and ordinary
/// strings containing escapes, are skipped.
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
/// <!-- docs: end -->
pub fn string_boundary(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let op_kind = operator?.kind();

    // Only check == and != operators
    if op_kind != RSyntaxKind::EQUAL2 && op_kind != RSyntaxKind::NOT_EQUAL {
        return Ok(None);
    }

    let left = left?;
    let right = right?;

    // Find a function call on either side of the comparison.
    let (call, string_expr) = if let AnyRExpression::RCall(c) = &left {
        (c, &right)
    } else if let AnyRExpression::RCall(c) = &right {
        (c, &left)
    } else {
        return Ok(None);
    };

    // Only check for calls to substr() and substring()
    let func_name = get_function_name(call.function()?);
    let formals: Formals = match func_name.as_str() {
        "substr" => &["x", "start", "stop"],
        "substring" => &["text", "first", "last"],
        _ => return Ok(None),
    };

    // Only check for calls with exactly 3 arguments
    if call.arguments()?.items().len() != 3 {
        return Ok(None);
    }

    // Only check for comparisons to non-empty string literals
    let width = unwrap_or_return_none!(literal_string_length(string_expr));

    // Bind the call arguments to the formal parameters and extract them
    let bound = CallContext::default().bind_arguments(call, formals);
    let x_arg = unwrap_or_return_none!(bound.get(formals[0]));
    let start_arg = unwrap_or_return_none!(bound.get(formals[1]));
    let end_arg = unwrap_or_return_none!(bound.get(formals[2]));

    // Get the string being compared
    let string_text = string_expr.syntax().text_trimmed();
    let x_text = x_arg.syntax().text_trimmed();

    // Check if the call is a substring at the start or end of the string
    let (replacement_fn, boundary) = if literal_integer(start_arg) == Some(1)
        && literal_integer(end_arg) == Some(width)
    {
        ("startsWith", "an initial")
    } else if is_nchar_of_same_expr(end_arg, x_arg) && is_suffix_start(start_arg, x_arg, width) {
        ("endsWith", "a terminal")
    } else {
        return Ok(None);
    };

    let range = ast.syntax().text_trimmed_range();
    let negation = if op_kind == RSyntaxKind::NOT_EQUAL {
        "!"
    } else {
        ""
    };
    let replacement = format!("{negation}{replacement_fn}({x_text}, {string_text})");
    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::StringBoundary,
            format!(
                "Using `{func_name}()` to detect {boundary} substring is hard to read and inefficient."
            ),
            Some(format!("Use `{replacement_fn}()` instead.")),
        ),
        range,
        Fix::new(range, replacement, node_contains_comments(ast.syntax())),
    )))
}

// Check if the expression is a string literal with a known length, returning None if it is not.
fn literal_string_length(expr: &AnyRExpression) -> Option<usize> {
    let string = expr.as_any_r_value()?.as_r_string_value()?;
    let content_token = string.content_token()?;
    let content = content_token.text_trimmed();
    let open = string.open_token().ok()?;
    let is_raw = open.text_trimmed().starts_with(['r', 'R']);

    // R normalizes source line endings; escapes also change the runtime length.
    // Skip both rather than count their source spelling.
    if content.is_empty() || content.contains('\r') || (!is_raw && content.contains('\\')) {
        return None;
    }
    Some(content.chars().count())
}

// Check if the expression is an integer literal, returning its value if it is, or None if it is not.
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

// Check if start_expr is nchar(x_expr) - (width - 1) where x_expr matches the first argument
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

// Check if end_expr is nchar(x_expr) where x_expr matches the first argument
fn is_nchar_of_same_expr(end_expr: &AnyRExpression, x_expr: &AnyRExpression) -> bool {
    let AnyRExpression::RCall(call) = end_expr else {
        return false;
    };
    let Ok(function) = call.function() else {
        return false;
    };
    if get_function_name(function) != "nchar"
        || !call.arguments().is_ok_and(|args| args.items().len() == 1)
    {
        return false;
    }
    get_arg(call, &["x"], "x")
        .and_then(|arg| arg.value())
        .is_some_and(|arg| arg.syntax().text_trimmed() == x_expr.syntax().text_trimmed())
}
