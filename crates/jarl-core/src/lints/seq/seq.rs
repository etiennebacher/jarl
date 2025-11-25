use crate::{diagnostic::*, utils::node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for `1:length(...)`, `1:nrow(...)`, `1:ncol(...)`, `1:NROW(...)` and
/// `1:NCOL(...)` expressions. See also [seq2](https://jarl.etiennebacher.com/rules/seq2).
///
/// ## Why is this bad?
///
/// Those patterns are often used to generate sequences from 1 to a given
/// number. However, when the right-hand side of `:` is 0, then this creates
/// a sequence `1,0` which is often overlooked.
///
/// This rule comes with safe automatic fixes using `seq_along()` or `seq_len()`.
///
/// ## Example
///
/// ```r
/// for (i in 1:nrow(data)) {
///   print("hi")
/// }
///
/// for (i in 1:length(data)) {
///   print("hi")
/// }
/// ```
///
/// Use instead:
/// ```r
/// for (i in seq_len(nrow(data))) {
///   print("hi")
/// }
///
/// for (i in seq_along(data)) {
///   print("hi")
/// }
/// ```
pub fn seq(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let operator = ast.operator()?;

    if operator.kind() != RSyntaxKind::COLON {
        return Ok(None);
    }

    let left = ast.left()?;
    let right = ast.right()?;

    let Some(right_call) = right.as_r_call() else {
        return Ok(None);
    };

    let left_is_literal_one = left.to_trimmed_text() == "1" || left.to_trimmed_text() == "1L";

    if !left_is_literal_one {
        return Ok(None);
    }

    let right_fun = right_call.function()?;
    let right_fun_name = right_fun.to_trimmed_string();
    if !["length", "nrow", "ncol", "NROW", "NCOL"].contains(&right_fun_name.as_str()) {
        return Ok(None);
    }

    let right_fun_content = right_call
        .arguments()?
        .items()
        .into_iter()
        .map(|x| x.unwrap().to_trimmed_string())
        .collect::<Vec<String>>()
        .join(", ");

    let (suggestion, replacement) = match right_fun_name.as_str() {
        "length" => (
            "seq_along(...)",
            format!("seq_along({})", right_fun_content),
        ),
        "nrow" => (
            "seq_len(nrow((...))",
            format!("seq_len(nrow({}))", right_fun_content),
        ),
        "ncol" => (
            "seq_len(ncol(...))",
            format!("seq_len(ncol({}))", right_fun_content),
        ),
        "NROW" => (
            "seq_len(NROW(...))",
            format!("seq_len(NROW({}))", right_fun_content),
        ),
        "NCOL" => (
            "seq_len(NCOL(...))",
            format!("seq_len(NCOL({}))", right_fun_content),
        ),
        // We checked the choices of right_fun_name above.
        _ => unreachable!(),
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "seq".to_string(),
            format!("`1:{right_fun_name}(...)` can be wrong if the RHS is 0.").to_string(),
            Some(format!("Use `{suggestion}` instead.").to_string()),
        ),
        range,
        Fix {
            content: replacement,
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
