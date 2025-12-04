use crate::diagnostic::*;
use crate::utils::{get_function_name, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for `substr()` and `substring()` calls that can be replaced with
/// `startsWith()` or `endsWith()`.
///
/// ## Why is this bad?
///
/// Using `startsWith()` and `endsWith()` is both more readable and more efficient
/// than extracting substrings and comparing them.
///
/// This rule has a safe fix.
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

    // Get arguments
    let arguments = call.arguments()?;
    let args: Vec<_> = arguments
        .items()
        .into_iter()
        .filter_map(|a| a.ok())
        .collect();

    // Need at least 3 arguments (x, start, end)
    if args.len() < 3 {
        return Ok(None);
    }

    // Extract the expression values from arguments
    let x_arg = unwrap_or_return_none!(args[0].value());
    let start_arg = unwrap_or_return_none!(args[1].value());
    let end_arg = unwrap_or_return_none!(args[2].value());

    // Get the string being compared
    let string_text = string_expr.syntax().text_trimmed();
    let x_text = x_arg.syntax().text_trimmed();

    // Check for startsWith pattern: start position is 1 or 1L
    if is_literal_one(&start_arg) {
        let range = ast.syntax().text_trimmed_range();

        // Build the replacement: startsWith(x, "string") or !startsWith(x, "string")
        let replacement = if op_kind == RSyntaxKind::NOT_EQUAL {
            format!("!startsWith({}, {})", x_text, string_text)
        } else {
            format!("startsWith({}, {})", x_text, string_text)
        };

        let diagnostic = Diagnostic::new(
            ViolationData::new(
                "string_boundary".to_string(),
                format!(
                    "Using `{func_name}()` to detect an initial substring is hard to read and inefficient."
                ),
                Some("Use `startsWith()` instead.".to_string()),
            ),
            range,
            Fix {
                content: replacement,
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        );
        return Ok(Some(diagnostic));
    }

    // Check for endsWith pattern: end position is nchar(x) where x is the same as the first arg
    if is_nchar_of_same_expr(&end_arg, &x_arg) {
        let range = ast.syntax().text_trimmed_range();

        // Build the replacement: endsWith(x, "string") or !endsWith(x, "string")
        let replacement = if op_kind == RSyntaxKind::NOT_EQUAL {
            format!("!endsWith({}, {})", x_text, string_text)
        } else {
            format!("endsWith({}, {})", x_text, string_text)
        };

        let diagnostic = Diagnostic::new(
            ViolationData::new(
                "string_boundary".to_string(),
                format!(
                    "Using `{func_name}()` to detect a terminal substring is hard to read and inefficient."
                ),
                Some("Use `endsWith()` instead.".to_string()),
            ),
            range,
            Fix {
                content: replacement,
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        );
        return Ok(Some(diagnostic));
    }

    Ok(None)
}

/// Check if an expression is the literal value 1 or 1L
fn is_literal_one(expr: &AnyRExpression) -> bool {
    // Check if it's an AnyRValue (numeric literal)
    if let Some(r_value) = expr.as_any_r_value() {
        // Check for integer value
        if let Some(int) = r_value.as_r_integer_value()
            && let Ok(token) = int.value_token()
        {
            let text = token.text_trimmed();
            return text == "1" || text == "1L" || text == "1l";
        }
        // Check for double value
        if let Some(double) = r_value.as_r_double_value()
            && let Ok(token) = double.value_token()
        {
            let text = token.text_trimmed();
            return text == "1" || text == "1.0" || text == "1.";
        }
    }
    false
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

    let args: Vec<_> = arguments
        .items()
        .into_iter()
        .filter_map(|a| a.ok())
        .collect();

    if args.len() != 1 {
        return false;
    }

    // Get the expression from the first argument
    let nchar_arg = match args[0].as_fields().value {
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
