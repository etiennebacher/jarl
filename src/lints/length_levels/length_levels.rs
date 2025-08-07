use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use air_r_syntax::*;
use anyhow::Result;
use biome_rowan::AstNode;
pub struct LengthLevels;

/// ## What it does
///
/// Check for `length(levels(...))` and replace it with `nlevels(...)`.
///
/// ## Why is this bad?
///
/// `length(levels(...))` is harder to read `nlevels(...)`.
///
/// Internally, `nlevels()` calls `length(levels(...))` so there are no
/// performance gains.
///
/// ## Example
///
/// ```r
/// x <- factor(1:3)
/// length(levels(x))
/// ```
///
/// Use instead:
/// ```r
/// x <- factor(1:3)
/// nlevels(x)
/// ```
impl Violation for LengthLevels {
    fn name(&self) -> String {
        "length_levels".to_string()
    }
    fn body(&self) -> String {
        "Use `nlevels(...)` instead of `length(levels(...))`.".to_string()
    }
}

impl LintChecker for LengthLevels {
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

        if outer_fn_name.text() != "length" {
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

            if inner_fn_name.text() != "levels" {
                return Ok(diagnostics);
            }

            let inner_content = arguments?.items().into_syntax().text();
            let range = ast.clone().into_syntax().text_trimmed_range();
            diagnostics.push(Diagnostic::new(
                LengthLevels,
                file,
                range,
                Fix {
                    content: format!("nlevels({})", inner_content),
                    start: range.start().into(),
                    end: range.end().into(),
                },
            ))
        }
        Ok(diagnostics)
    }
}
