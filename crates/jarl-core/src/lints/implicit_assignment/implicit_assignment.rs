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
pub fn implicit_assignment(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let operator = ast.operator()?;
    if operator.kind() != RSyntaxKind::ASSIGN
        && operator.kind() != RSyntaxKind::SUPER_ASSIGN
        && operator.kind() != RSyntaxKind::ASSIGN_RIGHT
        && operator.kind() != RSyntaxKind::SUPER_ASSIGN_RIGHT
    {
        return Ok(None);
    };

    // We want to report the use of assignment in function arguments, but not
    // when they're part of the body of some functions, e.g.
    // ```
    // local({
    //   x <- 1
    // })
    // ```
    // so we set `ancestor_is_arg = false` if an RBracedExpressions is a closer
    // ancestor.
    let ancestor_is_arg = {
        let mut result = false;
        for ancestor in ast.syntax().ancestors() {
            if RBracedExpressions::can_cast(ancestor.kind()) {
                // Found braced expressions first, so skip
                result = false;
                break;
            } else if RArgument::can_cast(ancestor.kind()) {
                // Found argument first, so include
                result = true;
                break;
            }
        }
        result
    };

    // Do not report cases like `expect_message(x <- foo)` because this code
    // could test that `<-.foo` returns a message, which is a valid usage.
    if ancestor_is_arg {
        for ancestor in ast.syntax().ancestors() {
            if RCall::can_cast(ancestor.kind()) {
                let function_name = RCall::cast(ancestor).unwrap().function()?.to_trimmed_text();
                if [
                    "expect_error",
                    "expect_warning",
                    "expect_message",
                    "expect_snapshot",
                    "quote",
                    "suppressMessages",
                    "suppressWarnings",
                ]
                .contains(&function_name.to_string().as_str())
                {
                    return Ok(None);
                } else {
                    break;
                }
            }
        }
    }

    // We want to skip cases like
    // ```r
    // if (TRUE) x <- 1
    // for (i in 1:2) x <- 1
    // while (TRUE) x <- 1
    // ```
    // i.e., we want to skip cases that are in the body of RIfStatement /
    // RForStatement / RWhileStatement.

    let ancestor_is_if = {
        let mut result = false;

        // The `consequence` part of an `RIfStatement` is always the 5th node
        // (index 4):
        // IF_KW - L_PAREN - [condition] - R_PAREN - [consequence]
        //
        // `.unwrap()` is fine here because the RBinaryExpression will always
        // have a parent.
        let in_if_body = ast.syntax().parent().unwrap().kind() == RSyntaxKind::R_IF_STATEMENT
            && ast.syntax().index() == 4;
        let in_else_body = ast.syntax().parent().unwrap().kind() == RSyntaxKind::R_ELSE_CLAUSE
            && ast.syntax().index() == 1;

        if !in_if_body && !in_else_body {
            for ancestor in ast.syntax().ancestors() {
                if RBracedExpressions::can_cast(ancestor.kind()) {
                    // Found braced expressions first, so skip
                    result = false;
                    break;
                } else if RIfStatement::can_cast(ancestor.kind()) {
                    // Found argument first, so include
                    result = true;
                    break;
                }
            }
        }
        result
    };
    let ancestor_is_while = {
        let mut result = false;

        // The `consequence` part of an `RWhileStatement` is always the 5th node
        // (index 4):
        // WHILE_KW - L_PAREN - [condition] - R_PAREN - [consequence]
        //
        // `.unwrap()` is fine here because the RBinaryExpression will always
        // have a parent.
        let in_while_body = ast.syntax().parent().unwrap().kind() == RSyntaxKind::R_WHILE_STATEMENT
            && ast.syntax().index() == 4;

        if !in_while_body {
            for ancestor in ast.syntax().ancestors() {
                if RBracedExpressions::can_cast(ancestor.kind()) {
                    // Found braced expressions first, so skip
                    result = false;
                    break;
                } else if RWhileStatement::can_cast(ancestor.kind()) {
                    // Found argument first, so include
                    result = true;
                    break;
                }
            }
        }
        result
    };
    let ancestor_is_for = {
        let mut result = false;

        // The `consequence` part of an `RWhileStatement` is always the 7th node
        // (index 6):
        // FOR_KW - L_PAREN - [value] - IN_KW - [sequence] - R_PAREN - [consequence]
        //
        // `.unwrap()` is fine here because the RBinaryExpression will always
        // have a parent.
        let in_for_body = ast.syntax().parent().unwrap().kind() == RSyntaxKind::R_FOR_STATEMENT
            && ast.syntax().index() == 6;

        if !in_for_body {
            for ancestor in ast.syntax().ancestors() {
                if RBracedExpressions::can_cast(ancestor.kind()) {
                    // Found braced expressions first, so skip
                    result = false;
                    break;
                } else if RForStatement::can_cast(ancestor.kind()) {
                    // Found argument first, so include
                    result = true;
                    break;
                }
            }
        }
        result
    };

    if !ancestor_is_if && !ancestor_is_while && !ancestor_is_arg && !ancestor_is_for {
        return Ok(None);
    }

    let msg = if ancestor_is_if {
        "Avoid implicit assignments in `if()` statements."
    } else if ancestor_is_while {
        "Avoid implicit assignments in `while()` statements."
    } else if ancestor_is_for {
        "Avoid implicit assignments in `for()` statements."
    } else if ancestor_is_arg {
        "Avoid implicit assignments in function calls."
    } else {
        unreachable!()
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new("implicit_assignment".to_string(), msg.to_string(), None),
        range,
        Fix::empty(),
    );

    Ok(Some(diagnostic))
}
