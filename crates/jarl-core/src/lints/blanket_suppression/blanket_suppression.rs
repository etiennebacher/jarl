use crate::diagnostic::*;
use air_r_syntax::*;
use biome_formatter::comments::Comments;
use biome_rowan::AstNode;

pub fn blanket_suppression(
    ast: &AnyRExpression,
    comments: &Comments<RLanguage>,
) -> anyhow::Result<Option<Diagnostic>> {
    if !ast.syntax().has_leading_comments() && !ast.syntax().has_trailing_comments() {
        return Ok(None);
    }

    let trailing_comments = comments.trailing_comments(ast.syntax());
    let dangling_comments = comments.dangling_comments(ast.syntax());
    let leading_comments = comments.leading_comments(ast.syntax());

    let all_comments = [leading_comments, trailing_comments, dangling_comments].concat();

    for comment in all_comments {
        let comment_content = comment.piece().text();
        if matches!(comment_content, "# nolint" | "#nolint") {
            let diagnostic = Diagnostic::new(
                ViolationData::new(
                    "blanket_suppression".to_string(),
                    "This comment suppresses all possible violations of this node.".to_string(),
                    Some(
                        "Consider ignoring specific rules instead, e.g., `# nolint: any_is_na`."
                            .to_string(),
                    ),
                ),
                comment.piece().text_range(),
                Fix::empty(),
            );

            return Ok(Some(diagnostic));
        }
    }

    Ok(None)
}
