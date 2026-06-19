use crate::diagnostic::*;
use crate::utils::get_function_name;
use air_r_syntax::*;
use biome_rowan::AstNode;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Reports `return()` used on the right-hand side of the `magrittr` pipe
/// `%>%`, whether written with parentheses (`x %>% return()`) or as a bare
/// identifier (`x %>% return`).
///
/// The native pipe `|>` is not considered because `x |> return()` is a syntax
/// error and would be caught by the parser anyway.
///
/// ## Why is this bad?
///
/// `return()` on the right-hand side of `%>%` does not behave like a regular
/// `return()` call and doesn't exit the function early. See the examples below.
///
/// ## Example
///
/// ```r
/// f <- function(x) {
///   x %>% sum() %>% return()
///   1 + 1
/// }
///
/// f(1:3)
/// #> 2
/// ```
///
/// In the example above, the output isn't the sum of `x` but `1 + 1`, even
/// though we'd expect the `return()` to return the output of `x %>% sum()`.
///
/// Wrapping the pipe in `return()` instead is unambiguous:
///
/// ```r
/// f <- function(x) {
///   return(x %>% sum())
///   1 + 1
/// }
///
/// # OR:
/// f <- function(x) {
///   out <- x %>% sum()
///   return(out)
///   1 + 1
/// }
///
/// f(1:3)
/// #> 6
/// ```
pub fn pipe_return(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left: _, operator, right } = ast.as_fields();
    let operator = operator?;
    let right = right?;

    // Only the magrittr pipe: `x |> return()` is a syntax error, so there is
    // nothing to flag for the native pipe.
    let is_magrittr_pipe =
        operator.kind() == RSyntaxKind::SPECIAL && operator.text_trimmed() == "%>%";
    if !is_magrittr_pipe {
        return Ok(None);
    }

    // The right-hand side must reference `return`. With `%>%`, the call can be
    // written with parentheses (`x %>% return()`) or without them as a bare
    // identifier (`x %>% return`); magrittr treats both the same way.
    let is_return = match &right {
        AnyRExpression::RCall(call) => get_function_name(call.function()?) == "return",
        AnyRExpression::RIdentifier(id) => id.name_token()?.text_trimmed() == "return",
        _ => false,
    };
    if !is_return {
        return Ok(None);
    }

    let range = right.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "pipe_return".to_string(),
            "Using `return()` after `%>%` doesn't actually return the output, which can create misleading results."
                .to_string(),
            Some("Either wrap the pipe in `return()` instead, or store the output in an intermediate object and use `return()` on it, e.g. `out <- x %>% sum(); return(out)`.".to_string()),
        ),
        range,
        Fix::empty(),
    );
    Ok(Some(diagnostic))
}
