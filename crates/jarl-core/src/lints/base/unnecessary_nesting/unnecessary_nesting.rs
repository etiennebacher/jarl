use crate::diagnostic::*;
use crate::utils::node_contains_comments;
use air_r_syntax::*;
use biome_rowan::{AstNode, AstNodeList};

/// ## What it does
///
/// This rule detects nested `if` conditions that could be gathered into a single
/// one.
///
/// ## Why is this bad?
///
/// Nesting `if` conditions when it is not necessary may hurt readability.
///
/// This rule has a safe fix, in the sense that it will not change the meaning
/// of the code. However, it may produce code that is incorrectly formatted.
///
/// ## Example
///
/// ```r
/// if (x > 0) {
///   if (y > 0) {
///     print("x and y are greather than 0")
///   }
/// }
/// ```
///
/// Use instead:
///
/// ```r
/// if (x > 0 && y > 0) {
///   print("x and y are greather than 0")
/// }
/// ```
pub fn unnecessary_nesting(ast: &RIfStatement) -> anyhow::Result<Option<Diagnostic>> {
    let body = ast.consequence()?;
    let has_else = ast.else_clause().is_some();

    if has_else {
        return Ok(None);
    }

    let outer_condition = ast.condition()?;
    let inner_if = unwrap_or_return_none!(body.as_r_braced_expressions());
    let inner_if = inner_if.expressions().iter().collect::<Vec<_>>();

    let inner_if = if inner_if.len() == 1 {
        inner_if.first().unwrap()
    } else {
        return Ok(None);
    };

    let inner_if = unwrap_or_return_none!(inner_if.as_r_if_statement());

    let has_inner_else = inner_if.else_clause().is_some();
    if has_inner_else {
        return Ok(None);
    }

    let inner_condition = inner_if.condition()?;
    let inner_consequence = inner_if.consequence()?;

    // Wrap conditions in parenthesis if they are more complex than a simple identifier.
    let outer_condition = if outer_condition.syntax().kind() == RSyntaxKind::R_IDENTIFIER {
        outer_condition.to_trimmed_string()
    } else {
        format!("({outer_condition})")
    };
    let inner_condition = if inner_condition.syntax().kind() == RSyntaxKind::R_IDENTIFIER {
        inner_condition.to_trimmed_string()
    } else {
        format!("({inner_condition})")
    };

    let replacement = format!(
        "if ({} && {}) {}",
        outer_condition,
        inner_condition,
        inner_consequence.to_trimmed_string()
    );

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "unnecessary_nesting".to_string(),
            "There is no need for nested if conditions here.".to_string(),
            Some("Gather the two conditions with `&&` instead.".to_string()),
        ),
        range,
        Fix {
            content: replacement,
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
    // Ok(None)
}
