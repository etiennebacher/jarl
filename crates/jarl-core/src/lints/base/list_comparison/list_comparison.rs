use crate::diagnostic::*;
use crate::utils::get_function_name;
use air_r_syntax::*;
use biome_rowan::AstNode;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for comparisons involving functions that are known to return lists,
/// such as `lapply(x, sum) > 10`.
///
/// ## Why is this bad?
///
/// These functions return lists, so R must coerce their results to vectors
/// before comparing them. This can hide the intended output type. Prefer a
/// mapper that returns a vector of the intended type directly, such as
/// `vapply()` or one of the typed `purrr::map_*()` functions.
///
/// This rule doesn't have an automatic fix because the expected output type
/// cannot be determined reliably from static analysis.
///
/// ## Example
///
/// ```r
/// lapply(x, sum) > 10
/// map(x, as.character) == "a"
/// ```
///
/// Use instead:
/// ```r
/// vapply(x, sum, numeric(1L)) > 10
/// map_chr(x, as.character) == "a"
/// ```
///
/// ## References
///
/// See `?lapply` and `?Comparison`.
pub fn list_comparison(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let operator = ast.operator()?;
    if !matches!(
        operator.kind(),
        RSyntaxKind::EQUAL2
            | RSyntaxKind::NOT_EQUAL
            | RSyntaxKind::GREATER_THAN
            | RSyntaxKind::GREATER_THAN_OR_EQUAL_TO
            | RSyntaxKind::LESS_THAN
            | RSyntaxKind::LESS_THAN_OR_EQUAL_TO
    ) {
        return Ok(None);
    }

    let left = ast.left()?;
    let right = ast.right()?;
    let mut mapper = None;

    for expression in [&left, &right] {
        let Some(call) = expression.as_r_call() else {
            continue;
        };

        let name = get_function_name(call.function()?);
        let suggestion = match name.as_str() {
            "lapply" => "Use `vapply()` with an explicit output type instead.",
            "map" => "Use a typed mapper such as `map_chr()` or `map_dbl()` instead.",
            "Map" | ".mapply" => "Use `mapply()` to return a vector directly.",
            _ => continue,
        };

        mapper = Some((name, suggestion));
        break;
    }

    let Some((mapper, suggestion)) = mapper else {
        return Ok(None);
    };

    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "list_comparison".to_string(),
            format!(
                "`{mapper}()` returns a list, so R must convert it to a vector before applying `{}`.",
                operator.text_trimmed()
            ),
            Some(suggestion.to_string()),
        ),
        ast.syntax().text_trimmed_range(),
        Fix::empty(),
    );

    Ok(Some(diagnostic))
}
