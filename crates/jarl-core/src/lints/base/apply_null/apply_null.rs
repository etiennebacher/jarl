use crate::checker::{Checker, PackageOrigin};
use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{Formals, StatementsExt, get_arg};
use air_r_syntax::*;
use biome_rowan::AstNode;

const FORMALS_LAPPLY: Formals = &["X", "FUN"];
const FORMALS_SAPPLY: Formals = &["X", "FUN"];
const FORMALS_MAP: Formals = &[".x", ".f"];

pub struct EmptyList;

/// <!-- docs: start -->
/// Version added: 0.8.0
///
/// ## What it does
///
/// Checks for `sapply()`, `lapply()` and `purrr::map()` calls where the
/// anonymous function returns only `NULL`.
///
/// Calls to `map()` without the `purrr::` prefix are only reported when
/// `map()` can be attributed to purrr (e.g. after `library(purrr)`).
///
/// ## Why is this bad?
///
/// These calls iterate over the input only to create a list of `NULL` values.
/// Creating an empty list of a given length with `vector()` is more efficient.
///
/// ## Example
///
/// ```r
/// lapply(x, function(x) NULL)
/// purrr::map(x, \(x) NULL)
/// purrr::map(x, ~NULL)
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
        "This way of creating an empty list can be inefficient.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `vector(\"list\", length(x))` to allocate the list directly.".to_string())
    }
}

pub fn apply_null(
    ast: &RCall,
    fn_name: &str,
    ns_prefix: Option<&str>,
    checker: &Checker,
) -> anyhow::Result<Option<Diagnostic>> {
    let (formals, fun_arg) = match (fn_name, ns_prefix) {
        ("lapply", None | Some("base::")) => (FORMALS_LAPPLY, "FUN"),
        ("sapply", None | Some("base::")) => (FORMALS_SAPPLY, "FUN"),
        ("map", Some("purrr::")) => (FORMALS_MAP, ".f"),
        ("map", None) if matches!(checker.resolve_package("map"), PackageOrigin::Resolved(pkg) if pkg == "purrr") => {
            (FORMALS_MAP, ".f")
        }
        _ => return Ok(None),
    };

    let Some(fun) = get_arg(ast, formals, fun_arg).and_then(|arg| arg.value()) else {
        return Ok(None);
    };

    let body_is_null = if let Some(function) = fun.as_r_function_definition() {
        body_returns_null(&function.body()?)
    } else if fn_name == "map" {
        // purrr also accepts formula lambdas, e.g. `~NULL`.
        fun.as_r_unary_expression().is_some_and(|formula| {
            formula
                .operator()
                .is_ok_and(|op| op.kind() == RSyntaxKind::TILDE)
                && formula.argument().is_ok_and(|arg| body_returns_null(&arg))
        })
    } else {
        false
    };
    if !body_is_null {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();
    Ok(Some(Diagnostic::new(EmptyList, range, Fix::empty())))
}

/// Whether a function body only returns `NULL`: `NULL`, `return()`,
/// `return(NULL)`, an empty `{}`, or one of those wrapped in `{ }`.
fn body_returns_null(body: &AnyRExpression) -> bool {
    if let Some(braced) = body.as_r_braced_statements() {
        let expressions = braced.expressions();
        expressions.is_empty()
            || (expressions.len() == 1 && expressions.first().is_some_and(returns_null))
    } else {
        returns_null(body)
    }
}

fn returns_null(expr: &AnyRExpression) -> bool {
    if expr.syntax().text_trimmed() == "NULL" {
        return true;
    }

    let Some(call) = expr.as_r_call() else {
        return false;
    };
    let Ok(function) = call.function() else {
        return false;
    };
    if !function.as_r_identifier().is_some_and(|id| {
        id.name_token()
            .is_ok_and(|token| token.text_trimmed() == "return")
    }) {
        return false;
    }

    let Ok(arguments) = call.arguments() else {
        return false;
    };
    let mut items = arguments.items().into_iter();
    match (items.next(), items.next()) {
        (None, None) => true,
        (Some(argument), None) => argument
            .ok()
            .and_then(|argument| argument.value())
            .is_some_and(|value| value.syntax().text_trimmed() == "NULL"),
        _ => false,
    }
}
