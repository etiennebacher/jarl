use crate::diagnostic::*;
use crate::utils::{
    drop_arg_by_name_or_position, get_arg_by_name_then_position, get_function_name,
    node_contains_comments,
};
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct SampleInt;

/// ## What it does
///
/// Checks for usage of `sample(1:n, m, ...)` and replaces it with
/// `sample.int(n, m, ...)` for readability.
///
/// ## Why is this bad?
///
/// `sample()` calls `sample.int()` internally so they have the same performance,
/// but the latter is more readable.
///
/// This rule is disabled by default.
///
/// ## Example
///
/// ```r
/// sample(1:10, 2)
/// ```
///
/// Use instead:
/// ```r
/// sample.int(10, 2)
/// ```
///
/// ## References
///
/// See `?sample`
impl Violation for SampleInt {
    fn name(&self) -> String {
        "sample_int".to_string()
    }
    fn body(&self) -> String {
        "`sample(1:n, m, ...)` is less readable than `sample.int(n, m, ...)`.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `sample.int(n, m, ...)` instead.".to_string())
    }
}

pub fn sample_int(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let RCallFields { function, arguments } = ast.as_fields();

    let function = function?;
    let fn_name = get_function_name(function);
    let args = arguments?.items();

    if fn_name != "sample" {
        return Ok(None);
    }

    let n = get_arg_by_name_then_position(&args, "n", 1);

    // Is the `n` argument of the form `1:x`? If so, keep the `x` part so it
    // can be reused in the fix.
    let right_value = if let Some(n) = n {
        let n_value = n.value().unwrap();
        if let Some(n_value) = n_value.as_r_binary_expression() {
            let RBinaryExpressionFields { left, operator, right } = n_value.as_fields();
            let left = left?;
            if left.to_trimmed_text() != "1" && left.to_trimmed_text() != "1L" {
                return Ok(None);
            }
            if operator?.kind() != RSyntaxKind::COLON {
                return Ok(None);
            }
            right?.to_trimmed_text().to_string()
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    };

    let other_args = drop_arg_by_name_or_position(&args, "n", 1);
    let inner_content = match other_args {
        Some(x) => {
            let out = x
                .iter()
                .map(|x| x.syntax().text_trimmed().to_string())
                .collect::<Vec<_>>()
                .join(", ");

            [right_value, out].join(", ")
        }
        None => right_value,
    };
    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        SampleInt,
        range,
        Fix {
            content: format!("sample.int({inner_content})"),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
