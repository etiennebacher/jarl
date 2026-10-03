use crate::diagnostic::*;
use crate::rule_set::Rule;
use air_r_syntax::RSyntaxNode;
use biome_rowan::Direction;

pub struct TodoComment;

/// <!-- docs: start -->
/// Version added: 0.7.0
///
/// ## What it does
///
/// Reports comments that start with `TODO` or `FIXME`, ignoring case. The marker
/// must follow the opening `#` characters, an optional roxygen `'`, and optional
/// whitespace.
///
/// A marker is reported when followed by the end of the comment, whitespace,
/// a symbol, or a digit. It is ignored when immediately followed by a Unicode
/// letter. For example, `# TODO123` and `# TODO: fix this` are reported, while
/// `# TODOLIST` and `# TODO中文` are ignored. Strings and markers in the middle
/// of comment text are ignored.
///
/// ## Why is this bad?
///
/// These comments can indicate unfinished work that should be reviewed before
/// releasing code.
///
/// This rule is disabled by default and has no automatic fix.
///
/// ## Example
///
/// ```r
/// # TODO: handle missing values
/// x <- 1 # FIXME
/// ```
///
/// Complete the work described by the comments, then remove the markers.
/// <!-- docs: end -->
impl Violation for TodoComment {
    fn rule(&self) -> Rule {
        Rule::TodoComment
    }

    fn body(&self) -> String {
        "Remove TODO comments.".to_string()
    }
}

pub fn todo_comment(syntax: &RSyntaxNode) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Comments are trivia attached to tokens, rather than expression nodes.
    for token in syntax.descendants_tokens(Direction::Next) {
        for piece in token
            .leading_trivia()
            .pieces()
            .chain(token.trailing_trivia().pieces())
        {
            if !piece.is_comments() {
                continue;
            }

            // Strip ordinary or roxygen comment prefixes, then match only at the start.
            let text = piece.text().trim_start_matches('#');
            let text = text.strip_prefix('\'').unwrap_or(text).trim_start();
            // `get` also rejects prefixes that would split a UTF-8 character.
            let Some(marker) = ["TODO", "FIXME"].into_iter().find(|marker| {
                text.get(..marker.len())
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case(marker))
            }) else {
                continue;
            };

            // Ignore TODOLIST and TODO中文, but still report TODO123 and TODO:.
            if text[marker.len()..]
                .chars()
                .next()
                .is_some_and(char::is_alphabetic)
            {
                continue;
            }

            diagnostics.push(Diagnostic::new(
                TodoComment,
                piece.text_range(),
                Fix::empty(),
            ));
        }
    }

    diagnostics
}
