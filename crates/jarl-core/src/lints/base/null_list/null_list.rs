use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{Formals, StatementsExt, get_arg, node_contains_comments};
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
/// The automated fix is marked unsafe because `vector("list", length(x))`
/// doesn't preserve names: `lapply()` and `purrr::map()` keep the names of
/// `x`, and `sapply()` also uses `x` itself as names when it is a character
/// vector. Jarl cannot know whether `x` has names, so they need to be added
/// back manually if they matter, e.g. with
/// `setNames(vector("list", length(x)), names(x))`.
/// <!-- docs: end -->
impl Violation for EmptyList {
    fn rule(&self) -> Rule {
        Rule::EmptyList
    }
    fn body(&self) -> String {
        "This way of creating an empty list is inefficient.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some(
            "Use `vector(\"list\", length(x))` to allocate the list directly. Names of `x` are not kept."
                .to_string(),
        )
    }
}

pub fn null_list(
    ast: &RCall,
    fn_name: &str,
    ns_prefix: Option<&str>,
) -> anyhow::Result<Option<Diagnostic>> {
    let (formals, x_arg, fun_arg) = match (fn_name, ns_prefix) {
        ("lapply", None | Some("base::")) => (FORMALS_LAPPLY, "X", "FUN"),
        ("sapply", None | Some("base::")) => (FORMALS_SAPPLY, "X", "FUN"),
        ("map", Some("purrr::")) => (FORMALS_MAP, ".x", ".f"),
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
    let fix = match get_arg(ast, formals, x_arg).and_then(|arg| arg.value()) {
        Some(x) => Fix::new(
            range,
            format!("vector(\"list\", length({}))", x.syntax().text_trimmed()),
            node_contains_comments(ast.syntax()),
        ),
        None => Fix::empty(),
    };
    Ok(Some(Diagnostic::new(EmptyList, range, fix)))
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

/// Whether a single expression evaluates to `NULL`. This is true for:
/// - `NULL`
/// - `return()`
/// - `return(NULL)` (also with a named argument, e.g. `return(value = NULL)`)
///
/// Other expressions, such as `return(x)` or `foo(NULL)`, return false.
fn returns_null(expr: &AnyRExpression) -> bool {
    if expr.syntax().text_trimmed() == "NULL" {
        return true;
    }

    if let Some(call) = expr.as_r_call()
        && let Ok(function) = call.function()
        && function.as_r_identifier().is_some_and(|id| {
            id.name_token()
                .is_ok_and(|token| token.text_trimmed() == "return")
        })
        && let Ok(arguments) = call.arguments()
    {
        let mut items = arguments.items().into_iter();
        match (items.next(), items.next()) {
            (None, None) => true,
            (Some(argument), None) => argument
                .ok()
                .and_then(|argument| argument.value())
                .is_some_and(|value| value.syntax().text_trimmed() == "NULL"),
            _ => false,
        }
    } else {
        false
    }
}
