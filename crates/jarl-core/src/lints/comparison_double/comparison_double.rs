use crate::diagnostic::*;
use crate::utils::node_contains_comments;
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for comparisons to a double value (aka float).
///
/// ## Why is this bad?
///
/// In some cases, floating point inacurracies can lead to unexpected results
/// when comparing two values that should be equal but are not, e.g.:
/// ```r
/// x <- 0.1 * 3
/// x == 0.3
/// #> [1] FALSE
/// ```
///
/// This rule has a safe fix that consists in using `all.equal()` when comparing
/// to doubles:
/// ```r
/// isTRUE(all.equal(x, 0.3))
/// #> [1] TRUE
/// ```
///
/// Note that `all.equal()` returns a character value if the equality does not
/// hold, which is why it is necessary to wrap it in `isTRUE()` to recover the
/// behavior of `==`.
///
/// ## Example
///
/// ```r
/// x == 1
/// f(x) == 1.3
/// ```
///
/// Use instead:
/// ```r
/// isTRUE(all.equal(x, 1))
/// isTRUE(all.equal(f(x), 1.3))
/// ```
///
/// # References
///
/// See:
///
/// - [R FAQ 7.31](https://cran.r-project.org/doc/FAQ/R-FAQ.html#Why-doesn_0027t-R-think-these-numbers-are-equal_003f)
/// - [https://stackoverflow.com/questions/9508518/why-are-these-numbers-not-equal/9508558](https://stackoverflow.com/questions/9508518/why-are-these-numbers-not-equal/9508558) (contains other links too)
pub fn comparison_double(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let operator = ast.operator()?;

    if operator.kind() != RSyntaxKind::EQUAL2 {
        return Ok(None);
    }
    let left = ast.left()?;
    let right = ast.right()?;

    let left_is_literal_double = match left.as_any_r_value() {
        Some(x) => x.as_r_double_value().is_some(),
        None => false,
    };
    let right_is_literal_double = match right.as_any_r_value() {
        Some(x) => x.as_r_double_value().is_some(),
        None => false,
    };

    let (replacement_x, replacement_y) = match (left_is_literal_double, right_is_literal_double) {
        (true, true) | (false, false) => return Ok(None),
        (true, false) => (right.to_trimmed_string(), left.to_trimmed_string()),
        (false, true) => (left.to_trimmed_string(), right.to_trimmed_string()),
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "comparison_double".to_string(),
            "Comparing to a double can lead to unexpected results because of floating point inacurracies.".to_string(),
            Some("Use `isTRUE(all.equal())` instead.".to_string()),
        ),
        range,
        Fix {
            content: format!(
                "isTRUE(all.equal({}, {}))",
                replacement_x,
                replacement_y
            ),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
