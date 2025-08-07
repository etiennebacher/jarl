use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use air_r_syntax::*;
use anyhow::Result;
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
        "`grep(pattern, x)` is better than `which(grepl(pattern, x))`.".to_string()
    }
}

impl LintChecker for WhichGrepl {
    fn check(&self, ast: &AnyRExpression, file: &str) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        let ast = if let Some(ast) = ast.as_r_call() {
            ast
        } else {
            return Ok(diagnostics);
        };
        let RCallFields { function, arguments } = ast.as_fields();

        let outer_fn_name = function?
            .as_r_identifier()
            .expect("In RCall, the function name must exist")
            .name_token()?
            .token_text_trimmed();

        if outer_fn_name.text() != "which" {
            return Ok(diagnostics);
        }

        let items = arguments?.items();

        let unnamed_arg = items
            .into_iter()
            .find(|x| x.clone().unwrap().name_clause().is_none());

        let value = unnamed_arg.unwrap()?.value();

        if let Some(inner) = value
            && let Some(inner2) = inner.as_r_call()
        {
            let RCallFields { function, arguments } = inner2.as_fields();

            let inner_fn_name = function?
                .as_r_identifier()
                .expect("In RCall, the function name must exist")
                .name_token()?
                .token_text_trimmed();

            if inner_fn_name.text() != "grepl" {
                return Ok(diagnostics);
            }

            let inner_content = arguments?.items().into_syntax().text();
            let range = ast.clone().into_syntax().text_trimmed_range();
            diagnostics.push(Diagnostic::new(
                WhichGrepl,
                file,
                range,
                Fix {
                    content: format!("grep({})", inner_content),
                    start: range.start().into(),
                    end: range.end().into(),
                },
            ))
        }
        Ok(diagnostics)
    }
}
