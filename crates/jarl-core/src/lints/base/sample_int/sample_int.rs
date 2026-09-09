use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{Formals, drop_arg, get_arg, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

const FORMALS_SAMPLE: Formals = &["x", "size", "replace", "prob"];

pub struct SampleInt;

/// Version added: 0.0.16
///
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
    fn rule(&self) -> Rule {
        Rule::SampleInt
    }
    fn body(&self) -> String {
        "`sample(1:n, m, ...)` is less readable than `sample.int(n, m, ...)`.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `sample.int(n, m, ...)` instead.".to_string())
    }
}

pub fn sample_int(ast: &RCall, fn_name: &str) -> anyhow::Result<Option<Diagnostic>> {
    if fn_name != "sample" {
        return Ok(None);
    }

    let n = get_arg(ast, FORMALS_SAMPLE, "x");

    // Is the `n` argument of the form `1:x`? If so, keep the `x` part so it
    // can be reused in the fix.
    let right_value = if let Some(n) = n {
        let n_value = unwrap_or_return_none!(n.value());
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

    let other_args = drop_arg(ast, FORMALS_SAMPLE, "x");
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
        Fix::new(
            range,
            format!("sample.int({inner_content})"),
            node_contains_comments(ast.syntax()),
        ),
    );

    Ok(Some(diagnostic))
}
