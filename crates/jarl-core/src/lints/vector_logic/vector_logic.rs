use crate::diagnostic::*;
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for implicit assignment in function calls and other situations.
///
/// ## Why is this bad?
///
/// Assigning inside function calls or other situations such as in `if()` makes
/// the code difficult to read, and should be avoided.
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

    let suggestion = match operator.kind() {
        RSyntaxKind::AND => format!("Use `&&` instead."),
        RSyntaxKind::OR => format!("Use `||` instead."),
        _ => unreachable!(),
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "vector_logic".to_string(),
            msg.to_string(),
            Some(suggestion),
        ),
        range,
        Fix::empty(),
    );

    Ok(Some(diagnostic))
}
