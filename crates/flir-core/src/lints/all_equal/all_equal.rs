use crate::diagnostic::*;
use crate::utils::{get_function_name, get_nested_functions_content, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for usage of `any(duplicated(...))`.
///
/// ## Why is this bad?
///
/// `any(duplicated(...))` is valid code but requires the evaluation of
/// `duplicated()` on the entire input first.
///
/// There is a more efficient function in base R called `anyDuplicated()` that
/// is more efficient, both in speed and memory used. `anyDuplicated()` returns
/// the index of the first duplicated value, or 0 if there is none.
///
/// Therefore, we can replace `any(duplicated(...))` by `anyDuplicated(...) > 0`.
///
/// ## Example
///
/// ```r
/// x <- c(1:10000, 1, NA)
/// any(duplicated(x))
/// ```
///
/// Use instead:
/// ```r
/// x <- c(1:10000, 1, NA)
/// anyDuplicated(x) > 0
/// ```
///
/// ## References
///
/// See `?anyDuplicated`
pub fn all_equal(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let fun_name = get_function_name(function);
    if fun_name != "all.equal" && fun_name != "isFALSE" {
        return Ok(None);
    }

    let mut msg = "".to_string();
    let mut fix_content = "".to_string();
    let mut range = ast.syntax().text_trimmed_range();

    let inner_content = get_nested_functions_content(ast, "isFALSE", "all.equal")?;
    if let Some(inner_content) = inner_content {
        let range = ast.syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(
            ViolationData::new("all_equal".to_string(), "Use `!isTRUE()` to check for differences in `all.equal()`. `isFALSE(all.equal())` always returns `FALSE`.".to_string()),
            range,
            Fix {
                content: format!("!isTRUE(all.equal({inner_content}))"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        );

        return Ok(Some(diagnostic));
    }

    // The `condition` part of an `RIfStatement` is always the 3rd node
    // (index 2):
    // IF_KW - L_PAREN - [condition] - R_PAREN - [consequence]
    let in_if_condition = ast.syntax().parent().unwrap().kind() == RSyntaxKind::R_IF_STATEMENT
        && ast.syntax().index() == 2;
    // The `consequence` part of an `RWhileStatement` is always the 3rd node
    // (index 2):
    // WHILE_KW - L_PAREN - [condition] - R_PAREN - [consequence]
    let in_while_condition = ast.syntax().parent().unwrap().kind()
        == RSyntaxKind::R_WHILE_STATEMENT
        && ast.syntax().index() == 2;
    if in_if_condition || in_while_condition {
        msg = "Wrap `all.equal()` in `isTRUE()`, or replace it by `identical()` if no tolerance is required.".to_string();
        fix_content = format!("isTRUE({})", ast.to_trimmed_text());
    }

    let prev_is_bang = if let Some(prev) = ast.syntax().prev_sibling_or_token() {
        prev.kind() == RSyntaxKind::BANG
    } else {
        false
    };
    if prev_is_bang {
        msg = "Wrap `all.equal()` in `isTRUE()`, or replace it by `identical()` if no tolerance is required.".to_string();
        fix_content = format!("!isTRUE({})", ast.to_trimmed_text());
        range = TextRange::new(
            ast.syntax()
                .prev_sibling_or_token()
                .unwrap()
                .text_trimmed_range()
                .start(),
            range.end(),
        )
    }

    if !msg.is_empty() {
        let diagnostic = Diagnostic::new(
            ViolationData::new("all_equal".to_string(), msg),
            range,
            Fix {
                content: fix_content,
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        );

        return Ok(Some(diagnostic));
    }

    Ok(None)
}
