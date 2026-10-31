use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{Formals, StatementsExt, get_arg};
use air_r_syntax::*;
use biome_rowan::AstNode;

const FORMALS_LAPPLY: Formals = &["X", "FUN"];
const FORMALS_SAPPLY: Formals = &["X", "FUN"];

pub struct EmptyList;

/// <!-- docs: start -->
/// Version added: 0.8.0
///
/// ## What it does
///
/// Checks for `sapply()` and `lapply()` calls where the anonymous function
/// returns only `NULL`.
///
/// ## Why is this bad?
///
/// These calls iterate over the input only to create a list of `NULL` values.
/// Allocating the list directly is more efficient.
///
/// ## Example
///
/// ```r
/// lapply(x, function(x) NULL)
/// ```
///
/// Use instead:
/// ```r
/// vector("list", length(x))
/// ```
///
/// This suggestion does not preserve names from `x`.
/// <!-- docs: end -->
impl Violation for EmptyList {
    fn rule(&self) -> Rule {
        Rule::EmptyList
    }
    fn body(&self) -> String {
        "This call iterates over its input to create a list of `NULL` values.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `vector(\"list\", length(x))` to allocate the list directly.".to_string())
    }
}

pub fn apply_null(
    ast: &RCall,
    fn_name: &str,
    ns_prefix: Option<&str>,
) -> anyhow::Result<Option<Diagnostic>> {
    if !matches!(fn_name, "lapply" | "sapply") || !matches!(ns_prefix, None | Some("base::")) {
        return Ok(None);
    }

    let formals = if fn_name == "lapply" {
        FORMALS_LAPPLY
    } else {
        FORMALS_SAPPLY
    };
    let Some(fun) = get_arg(ast, formals, "FUN").and_then(|arg| arg.value()) else {
        return Ok(None);
    };
    let Some(function) = fun.as_r_function_definition() else {
        return Ok(None);
    };
    let body = function.body()?;

    let body_is_null = if let Some(braced) = body.as_r_braced_statements() {
        let expressions = braced.expressions();
        expressions.len() == 1
            && expressions
                .first()
                .is_some_and(|expr| expr.syntax().text_trimmed().to_string() == "NULL")
    } else {
        body.syntax().text_trimmed().to_string() == "NULL"
    };
    if !body_is_null {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();
    Ok(Some(Diagnostic::new(EmptyList, range, Fix::empty())))
}
