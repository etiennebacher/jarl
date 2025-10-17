use crate::{diagnostic::*, utils::node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct Repeat;

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
impl Violation for Repeat {
    fn name(&self) -> String {
        "repeat".to_string()
    }
    fn body(&self) -> String {
        "Use `repeat` instead of `while (TRUE)` for infinite loops.".to_string()
    }
}

pub fn repeat(ast: &RWhileStatement) -> anyhow::Result<Option<Diagnostic>> {
    let condition = ast.condition()?;

    if condition.as_r_true_expression().is_some() {
        let body = ast.body()?;
        let body_text = body.to_trimmed_text();

        let is_braced = body.as_r_braced_expressions().is_some();
        let fix_content = if is_braced {
            format!("{body_text}")
        } else {
            format!("{{ {body_text} }}")
        };

        let range = ast.syntax().text_trimmed_range();
        let range_to_report = TextRange::new(
            ast.while_token()?.text_trimmed_range().start(),
            ast.r_paren_token()?.text_trimmed_range().end(),
        );

        let diagnostic = Diagnostic::new(
            Repeat,
            range_to_report,
            Fix {
                content: format!("repeat {fix_content}"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        );
        return Ok(Some(diagnostic));
    }

    Ok(None)
}
