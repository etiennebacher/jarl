use crate::diagnostic::*;
use crate::utils::{get_nested_functions_content, node_contains_comments};
use air_r_syntax::*;

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
        "any_duplicated".to_string()
    }
    fn body(&self) -> String {
        "`any(duplicated(...))` is inefficient.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `anyDuplicated(...) > 0` instead.".to_string())
    }
}

pub fn any_duplicated(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let (inner_content, outer_syntax) =
        unwrap_or_return_none!(get_nested_functions_content(ast, "any", "duplicated")?);

    let range = outer_syntax.text_trimmed_range();
    Ok(Some(Diagnostic::new(
        AnyDuplicated,
        range,
        Fix {
            content: format!("anyDuplicated({inner_content}) > 0"),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(&outer_syntax),
        },
    )))
}
