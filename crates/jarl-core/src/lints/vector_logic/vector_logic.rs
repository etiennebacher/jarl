use crate::diagnostic::*;
use crate::utils::get_function_name;
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for calls to `&` and `|` in the conditions of `if` and `while`
/// statements.
///
/// ## Why is this bad?
///
/// Usage of `&` and `|` in conditional statements is error-prone and inefficient.
/// Having a `condition` of length > 1 in those cases was causing a warning in
/// R 4.2.* and throws an error since R 4.3.0.
///
/// This rule only reports cases where the binary expression is the top operation
/// of the `condition`. For example, `if (x & y)` will be reported but
/// `if (foo(x & y))` will not.
///
/// ## Example
///
/// ```r
/// mean(x <- c(1, 2, 3))
/// x
///
/// if (any(y <- x > 0)) {
///   print(y)
/// }
/// ```
///
/// Use instead:
/// ```r
/// x <- c(1, 2, 3)
/// mean(x)
/// x
///
/// larger <- x > 0
/// if (any(larger)) {
///   print(larger)
/// }
/// ```
///
/// ## References
///
/// See:
///
/// - [https://style.tidyverse.org/syntax.html#assignment](https://style.tidyverse.org/syntax.html#assignment)
pub fn vector_logic(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let operator = ast.operator()?;
    if operator.kind() != RSyntaxKind::AND && operator.kind() != RSyntaxKind::OR {
        return Ok(None);
    };

    // Exception: bitwise operations with raw/octmode/hexmode or string literals
    // See https://github.com/r-lib/lintr/issues/1453
    let left = ast.left()?;
    let right = ast.right()?;
    if is_bitwise_exception(&left) || is_bitwise_exception(&right) {
        return Ok(None);
    }

    // We want to only keep cases that are in the condition of RIfStatement or RWhileStatement.
    // In this rule, we don't want to go back to all ancestors, only the direct parent, because
    // we want to allow, e.g.:
    // ```r
    // if (x && any(x | y)) 1
    // ```

    // The `condition` part of an `RIfStatement` is always the 3rd node (index 2):
    // IF_KW - L_PAREN - [condition] - R_PAREN - [consequence]
    //
    // `.unwrap()` is fine here because the RBinaryExpression will always
    // have a parent.
    let parent_is_if_condition = ast.syntax().parent().unwrap().kind()
        == RSyntaxKind::R_IF_STATEMENT
        && ast.syntax().index() == 2;

    // The `condition` part of an `RWhileStatement` is always the 3rd node (index 2):
    // WHILE_KW - L_PAREN - [condition] - R_PAREN - [consequence]
    //
    // `.unwrap()` is fine here because the RBinaryExpression will always
    // have a parent.
    let parent_is_while_condition = ast.syntax().parent().unwrap().kind()
        == RSyntaxKind::R_WHILE_STATEMENT
        && ast.syntax().index() == 2;

    if !parent_is_while_condition && !parent_is_if_condition {
        return Ok(None);
    }

    let msg = if parent_is_if_condition {
        format!(
            "`{}` in `if()` statements can lead to conditions of length > 1, which will error.",
            operator.text_trimmed()
        )
    } else if parent_is_while_condition {
        format!(
            "`{}` in `while()` statements can lead to conditions of length > 1, which will error.",
            operator.text_trimmed()
        )
    } else {
        unreachable!()
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new("vector_logic".to_string(), msg.to_string(), None),
        range,
        Fix::empty(),
    );

    Ok(Some(diagnostic))
}

/// Check if an expression is a raw/octmode/hexmode call or a string literal
fn is_bitwise_exception(expr: &AnyRExpression) -> bool {
    // Check for as.raw(), as.octmode(), as.hexmode() calls
    if let Some(call) = expr.as_r_call() {
        if let Ok(function) = call.function() {
            let fn_name = get_function_name(function);
            if fn_name == "as.raw" || fn_name == "as.octmode" || fn_name == "as.hexmode" {
                return true;
            }
        }
    }

    // Check for string literals (implicit as.octmode coercion)
    if let Some(val) = expr.as_any_r_value() {
        if val.as_r_string_value().is_some() {
            return true;
        }
    }

    false
}
