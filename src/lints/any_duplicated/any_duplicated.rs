use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;
use anyhow::{Context, Result};

pub struct AnyDuplicated;

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
impl Violation for AnyDuplicated {
    fn name(&self) -> String {
        "any-duplicated".to_string()
    }
    fn body(&self) -> String {
        "`any(duplicated(...))` is inefficient. Use `anyDuplicated(...) > 0` instead.".to_string()
    }
}

impl LintChecker for AnyDuplicated {
    fn check(&self, ast: &RCall, file: &str) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = vec![];

        let RCallFields { function, arguments } = ast.as_fields();

        let outer_fn_name = function?
            .as_r_identifier()
            .expect("In RCall, the function name must exist")
            .name_token()?
            .token_text_trimmed();

        if outer_fn_name.text() != "any" {
            return Ok(diagnostics);
        }

        let items = arguments?.items();

        let unnamed_arg = items
            .into_iter()
            .find(|x| x.clone().unwrap().name_clause().is_none());

        // any(na.rm = TRUE/FALSE) and any() are valid
        if unnamed_arg.is_none() {
            return Ok(diagnostics);
        }

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

            if inner_fn_name.text() != "duplicated" {
                return Ok(diagnostics);
            }

            use biome_rowan::AstNode;
            let inner_content = arguments?.items().into_syntax().text();
            let range = ast.clone().into_syntax().text_trimmed_range();
            diagnostics.push(Diagnostic::new(
                AnyDuplicated,
                file,
                range,
                Fix {
                    content: format!("anyDuplicated({}) > 0", inner_content),
                    start: range.start().into(),
                    end: range.end().into(),
                },
            ))
        }
        Ok(diagnostics)
    }
}
