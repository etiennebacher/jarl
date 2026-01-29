use crate::diagnostic::*;
use air_r_syntax::*;
use biome_formatter::comments::Comments;
use biome_rowan::{AstNode, TextRange};

// Define jarl-ignore variants as a constant
const IGNORE_JARL_VARIANTS: &[&str] = &["# jarl-ignore", "#jarl-ignore"];

pub fn blanket_suppression(
    ast: &AnyRExpression,
    comments: &Comments<RLanguage>,
) -> anyhow::Result<Option<Diagnostic>> {
    let syntax = ast.syntax();

    // Early exit: most nodes don't have comments
    if !syntax.has_leading_comments() && !syntax.has_trailing_comments() {
        return Ok(None);
    }

    // Check each comment type separately (avoid concat allocation)
    // Check trailing comments first (most common for jarl-ignore)
    for comment in comments.trailing_comments(syntax) {
        let text = comment.piece().text();
        if IGNORE_JARL_VARIANTS.contains(&text) {
            return Ok(Some(create_diagnostic(comment.piece().text_range())));
        }
    }

    // Check leading comments
    for comment in comments.leading_comments(syntax) {
        let text = comment.piece().text();
        if IGNORE_JARL_VARIANTS.contains(&text) {
            return Ok(Some(create_diagnostic(comment.piece().text_range())));
        }
    }

    // Check dangling comments (least common)
    for comment in comments.dangling_comments(syntax) {
        let text = comment.piece().text();
        if IGNORE_JARL_VARIANTS.contains(&text) {
            return Ok(Some(create_diagnostic(comment.piece().text_range())));
        }
    }

    Ok(None)
}

/// Create diagnostic for blanket suppression
#[inline]
fn create_diagnostic(range: TextRange) -> Diagnostic {
    Diagnostic::new(
        ViolationData::new(
            "blanket_suppression".to_string(),
            "This comment isn't used by Jarl because it suppresses all possible violations of this node.".to_string(),
            Some(
                "Use targeted comments instead, e.g., `# jarl-ignore any_is_na: <explanation>`."
                    .to_string(),
            ),
        ),
        range,
        Fix::empty(),
    )
}
