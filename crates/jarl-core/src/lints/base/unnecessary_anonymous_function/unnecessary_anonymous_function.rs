use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::node_contains_comments;
use air_r_syntax::*;
use biome_rowan::{AstNode, AstNodeList, AstSeparatedList};
use oak_core::syntax_ext::RIdentifierExt;

/// Functions where `FUN` is directly followed by `...`, which is passed on to
/// `FUN`. This excludes e.g. `Reduce()` (no `...`) or `sweep()` (arguments
/// between `FUN` and `...`).
const APPLY_FUNS: &[&str] = &[
    "lapply",
    "sapply",
    "vapply",
    "apply",
    "tapply",
    "rapply",
    "eapply",
    "dendrapply",
    "mapply",
    "by",
    "mclapply",
    "mcmapply",
    "parApply",
    "parCapply",
    "parLapply",
    "parLapplyLB",
    "parRapply",
    "parSapply",
    "parSapplyLB",
    "pvec",
];

const PURRR_MAPPERS: &[&str] = &[
    "map", "walk", "map_raw", "map_lgl", "map_int", "map_dbl", "map_chr", "map_vec", "map_df",
    "map_dfr", "map_dfc",
];

/// Arguments of the outer call that are not passed to `FUN`.
const MAPPLY_OPTIONS: &[&str] = &[
    "MoreArgs",
    "SIMPLIFY",
    "USE.NAMES",
    "mc.preschedule",
    "mc.set.seed",
    "mc.silent",
    "mc.cores",
    "mc.cleanup",
    "affinity.list",
];
const PURRR_OPTIONS: &[&str] = &[".progress", ".ptype", ".id"];

/// <!-- docs: start -->
/// Version added: unreleased
///
/// ## What it does
///
/// Checks for anonymous functions in `lapply()`, `sapply()`, `purrr::map()`,
/// and similar functions, that only call another function, e.g.
/// `lapply(x, function(xi) sum(xi))`.
///
/// ## Why is this bad?
///
/// The anonymous function is unnecessary: the function it calls can be passed
/// directly, e.g. `lapply(x, sum)`.
///
/// Anonymous functions that do more than calling another function, e.g.
/// `sapply(x, function(xi) foo(xi) == 2)`, are not reported. Neither are
/// anonymous functions whose first argument isn't the iterated element, e.g.
/// `lapply(x, function(xi) grep("a", xi))`, or that pass extra arguments to the
/// inner function, e.g. `lapply(x, function(xi) sum(xi, na.rm = TRUE))` or
/// `lapply(x, function(xi, y) foo(xi, y), y)`, because removing them is not
/// always more readable.
///
/// This rule has an unsafe automatic fix because the inner function may use
/// non-standard evaluation, e.g. `function(x) substitute(x)`.
///
/// ## Example
///
/// ```r
/// lapply(x, function(xi) sum(xi))
/// purrr::map(x, ~ sum(.x))
/// ```
///
/// Use instead:
/// ```r
/// lapply(x, sum)
/// purrr::map(x, sum)
/// ```
/// <!-- docs: end -->
pub fn unnecessary_anonymous_function(
    ast: &RCall,
    fn_name: &str,
) -> anyhow::Result<Option<Diagnostic>> {
    let is_purrr = PURRR_MAPPERS.contains(&fn_name);
    if !is_purrr && !APPLY_FUNS.contains(&fn_name) {
        return Ok(None);
    }

    let outer_args: Vec<RArgument> = ast.arguments()?.items().iter().flatten().collect();
    let options: &[&str] = match fn_name {
        "mapply" | "mcmapply" => MAPPLY_OPTIONS,
        _ if is_purrr => PURRR_OPTIONS,
        _ => &[],
    };
    let n_options = outer_args
        .iter()
        .filter(|a| argument_name(a).is_some_and(|n| options.contains(&n.as_str())))
        .count();
    // In `x |> lapply(f)`, `x` is an argument too.
    let n_other_args =
        (outer_args.len() - n_options + usize::from(is_piped(ast))).saturating_sub(1);

    for arg in &outer_args {
        let Some(value) = arg.value() else {
            continue;
        };
        let diagnostic = if let Some(lambda) = value.as_r_function_definition() {
            check_lambda(lambda, fn_name, &outer_args, n_other_args)
        } else if is_purrr && let Some(formula) = value.as_r_unary_expression() {
            check_formula(formula, fn_name, &outer_args, n_other_args)
        } else {
            None
        };
        if diagnostic.is_some() {
            return Ok(diagnostic);
        }
    }

    Ok(None)
}

fn check_lambda(
    lambda: &RFunctionDefinition,
    fn_name: &str,
    outer_args: &[RArgument],
    n_other_args: usize,
) -> Option<Diagnostic> {
    let mut formals = vec![];
    for param in lambda.parameters().ok()?.items().iter() {
        let param = param.ok()?;
        if param.default().is_some() {
            return None;
        }
        formals.push(symbol_name(param.name().ok()?.syntax())?);
    }
    // Formals beyond the iterated elements receive extra arguments of the outer
    // call, e.g. `lapply(x, function(xi, y) foo(xi, y), y)`, which is clearer
    // than `lapply(x, foo, y)`.
    let n_iterated = match fn_name {
        "mapply" | "mcmapply" => usize::MAX,
        _ => 1,
    };
    if formals.is_empty() || formals.len() > n_iterated {
        return None;
    }
    // Formals other than `...` must be supplied by the outer call.
    if formals.iter().filter(|f| *f != "...").count() > n_other_args {
        return None;
    }
    // `mapply()` passes argument names on to `FUN`, so a name that isn't
    // exactly a formal (e.g. partially matched) would reach the inner function.
    // The first formal is always passed positionally to the inner function.
    if n_iterated == usize::MAX
        && outer_args.iter().filter_map(argument_name).any(|name| {
            name != "FUN"
                && !MAPPLY_OPTIONS.contains(&name.as_str())
                && (name == formals[0] || !formals.contains(&name))
        })
    {
        return None;
    }

    let body = unwrap_body(lambda.body().ok()?);
    let fun = inner_call_target(body.as_r_call()?, &formals, outer_args)?;
    Some(pass_directly_diagnostic(lambda.syntax(), &fun, fn_name))
}

/// purrr-style lambdas, e.g. `~ foo(.x)`.
fn check_formula(
    formula: &RUnaryExpression,
    fn_name: &str,
    outer_args: &[RArgument],
    n_other_args: usize,
) -> Option<Diagnostic> {
    if formula.operator().ok()?.kind() != RSyntaxKind::TILDE {
        return None;
    }
    // Formulas ignore extra arguments, e.g. `map(x, ~ round(.x), 2)` doesn't
    // round to 2 digits, but `map(x, round, 2)` does.
    if n_other_args > 1 {
        return None;
    }
    let body = unwrap_body(formula.argument().ok()?);
    let call = body.as_r_call()?;
    let first = call.arguments().ok()?.items().iter().next()?.ok()?;
    let first = symbol_name(first.value()?.syntax())?;
    if first != ".x" && first != "." {
        return None;
    }
    let fun = inner_call_target(call, &[first], outer_args)?;
    Some(pass_directly_diagnostic(formula.syntax(), &fun, fn_name))
}

/// If `call` can replace the anonymous function, return the text of its
/// function.
///
/// The arguments of `call` must be exactly `formals` in order, the first one
/// passed positionally and the others passed the same way as in the outer
/// call (positionally or by matching name).
fn inner_call_target(call: &RCall, formals: &[String], outer_args: &[RArgument]) -> Option<String> {
    let function = call.function().ok()?;
    if function.as_r_identifier().is_none() && function.as_r_namespace_expression().is_none() {
        return None;
    }
    let fun = function.syntax().text_trimmed().to_string();
    if fun == "return" {
        return None;
    }

    let inner_args: Vec<RArgument> = call.arguments().ok()?.items().iter().flatten().collect();
    if inner_args.len() != formals.len() {
        return None;
    }

    for (i, (arg, formal)) in inner_args.iter().zip(formals).enumerate() {
        if symbol_name(arg.value()?.syntax()).as_ref() != Some(formal) {
            return None;
        }
        let inner_name = arg.name_clause().and_then(|n| n.name().ok());
        let named_in_outer = outer_args
            .iter()
            .any(|a| argument_name(a).as_ref() == Some(formal));
        match inner_name {
            None if i == 0 || !named_in_outer => {}
            Some(name)
                if i > 0
                    && named_in_outer
                    && symbol_name(name.syntax()).as_ref() == Some(formal) => {}
            _ => return None,
        }
    }

    Some(fun)
}

fn pass_directly_diagnostic(lambda: &RSyntaxNode, fun: &str, fn_name: &str) -> Diagnostic {
    let range = lambda.text_trimmed_range();
    Diagnostic::new(
        ViolationData::new(
            Rule::UnnecessaryAnonymousFunction,
            format!("Unnecessary anonymous function around `{fun}()`."),
            Some(format!("Pass `{fun}` directly to `{fn_name}()`.")),
        ),
        range,
        Fix::new(range, fun.to_string(), node_contains_comments(lambda)),
    )
}

/// Remove parentheses, braces around a single expression, and `return()`.
fn unwrap_body(mut expr: AnyRExpression) -> AnyRExpression {
    loop {
        let inner = if let Some(paren) = expr.as_r_parenthesized_expression() {
            paren.body().ok()
        } else if let Some(braced) = expr.as_r_braced_statements() {
            let statements = braced.statements();
            match statements.iter().next() {
                Some(stmt) if statements.len() == 1 => stmt
                    .as_r_expression_statement()
                    .and_then(|s| s.expression().ok()),
                _ => None,
            }
        } else if let Some(call) = expr.as_r_call()
            && call
                .function()
                .is_ok_and(|f| f.syntax().text_trimmed() == "return")
            && let Ok(args) = call.arguments()
            && args.items().len() == 1
            && let Some(Ok(arg)) = args.items().iter().next()
            && arg.name_clause().is_none()
        {
            arg.value()
        } else {
            None
        };
        match inner {
            Some(inner) => expr = inner,
            None => return expr,
        }
    }
}

/// Name of an identifier or `...`.
fn symbol_name(node: &RSyntaxNode) -> Option<String> {
    if let Some(id) = RIdentifier::cast_ref(node) {
        Some(id.name_text())
    } else if RDots::can_cast(node.kind()) {
        Some("...".to_string())
    } else {
        None
    }
}

fn argument_name(arg: &RArgument) -> Option<String> {
    symbol_name(arg.name_clause()?.name().ok()?.syntax())
}

fn is_piped(call: &RCall) -> bool {
    call.syntax()
        .parent()
        .and_then(RBinaryExpression::cast)
        .is_some_and(|binary| {
            binary.right().is_ok_and(|r| r.syntax() == call.syntax())
                && binary.operator().is_ok_and(|op| {
                    op.kind() == RSyntaxKind::PIPE
                        || (op.kind() == RSyntaxKind::SPECIAL && op.text_trimmed() == "%>%")
                })
        })
}
