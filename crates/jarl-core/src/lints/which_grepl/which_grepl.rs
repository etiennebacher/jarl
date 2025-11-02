use crate::diagnostic::*;
use crate::utils::{get_nested_functions_content, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct WhichGrepl;

/// ## What it does
///
/// Checks for usage of `which(grepl(...))` and replaces it with `grep(...)`.
///
/// ## Why is this bad?
///
/// `which(grepl(...))` is harder to read and is less efficient than `grep()`
/// since it requires two passes on the vector.
///
/// ## Example
///
/// ```r
/// x <- c("hello", "there")
/// which(grepl("hell", x))
/// which(grepl("foo", x))
/// ```
///
/// Use instead:
/// ```r
/// x <- c("hello", "there")
/// grep("hell", x)
/// grep("foo", x)
/// ```
///
/// ## References
///
/// See `?grep`
impl Violation for WhichGrepl {
    fn name(&self) -> String {
        "which_grepl".to_string()
    }
    fn body(&self) -> String {
        "`which(grepl(pattern, x))` is less efficient than `grep(pattern, x)`.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `grep(pattern, x)` instead.".to_string())
    }
}

pub fn which_grepl(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let inner_content = get_nested_functions_content(ast, "which", "grepl")?;

    if let Some(inner_content) = inner_content {
        let range = ast.syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(
            WhichGrepl,
            range,
            Fix {
                content: format!("grep({inner_content})"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        );
        return Ok(Some(diagnostic));
    }
    Ok(None)
}
