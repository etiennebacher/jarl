use crate::diagnostic::*;
use crate::rule_set::Rule;
use air_r_syntax::*;
use biome_rowan::{AstNode, Direction};

pub struct Repeat;

/// <!-- docs: start -->
/// Version added: 0.0.19
///
/// ## What it does
///
/// Checks use of `while (TRUE)` and recommends the use of `repeat` instead.
///
/// ## Why is this bad?
///
/// `while (TRUE)` is valid R code but `repeat` better expresses the intent of
/// infinite loop.
///
/// ## Example
///
/// ```r
/// while (TRUE) {
///   # ...
///   break
/// }
/// ```
///
/// Use instead:
/// ```r
/// repeat {
///   # ...
///   break
/// }
/// ```
/// <!-- docs: end -->
impl Violation for Repeat {
    fn rule(&self) -> Rule {
        Rule::Repeat
    }
    fn body(&self) -> String {
        "`while (TRUE)` is less clear than `repeat` for infinite loops.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `repeat {}` instead.".to_string())
    }
}

pub fn repeat(ast: &RWhileStatement) -> anyhow::Result<Option<Diagnostic>> {
    let mut condition = ast.condition()?;
    while let AnyRExpression::RParenthesizedExpression(paren) = condition {
        condition = paren.body()?;
    }

    if condition.as_r_true_expression().is_none() {
        return Ok(None);
    }

    let while_token = ast.while_token()?;
    let r_paren = ast.r_paren_token()?;
    let range = TextRange::new(
        while_token.text_trimmed_range().start(),
        r_paren.text_trimmed_range().end(),
    );

    // Only comments inside the replaced header prevent a fix; the body is untouched.
    let has_comments = ast
        .syntax()
        .descendants_tokens(Direction::Next)
        .take_while(|token| token.text_trimmed_range().start() < range.end())
        .any(|token| {
            (token != while_token && token.has_leading_comments())
                || (token != r_paren && token.has_trailing_comments())
        });

    // Keep existing whitespace, adding a separator only when the body is adjacent.
    let replacement = if ast.body()?.range().start() == range.end() {
        "repeat "
    } else {
        "repeat"
    };
    Ok(Some(Diagnostic::new(
        Repeat,
        range,
        Fix::new(range, replacement.to_string(), has_comments),
    )))
}
