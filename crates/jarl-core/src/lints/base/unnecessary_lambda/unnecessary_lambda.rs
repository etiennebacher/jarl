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
    "outer",
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

/// <!-- docs: start -->
/// Version added: 0.8.0
///
/// ## What it does
///
/// Checks for anonymous functions in `lapply()`, `sapply()`, `purrr::map()`,
/// and similar functions, that only call another function, e.g.
/// `lapply(x, function(xi) sum(xi))`.
///
/// It also checks for anonymous functions in `sapply()` and `vapply()` that
/// only compare the result of a function to a constant, e.g.
/// `sapply(x, function(xi) foo(xi) == 2)`.
///
/// ## Why is this bad?
///
/// The anonymous function is unnecessary: the function it calls can be passed
/// directly, e.g. `lapply(x, sum)`. Extra arguments can be passed after it,
/// e.g. `lapply(x, sum, na.rm = TRUE)`.
///
/// Comparing to a constant once on the output of `sapply()` or `vapply()`,
/// e.g. `sapply(x, foo) == 2`, is faster than doing it for every element since
/// `==` is vectorized.
///
/// Anonymous functions whose first argument isn't the iterated element, e.g.
/// `lapply(x, function(xi) grep("a", xi))`, are not reported because removing
/// them is not always more readable.
///
/// This rule has an unsafe automatic fix when the anonymous function only
/// passes its arguments to the inner function. It is unsafe because the inner
/// function may use non-standard evaluation, e.g. `function(x) substitute(x)`.
///
/// ## Example
///
/// ```r
/// lapply(x, function(xi) sum(xi))
/// lapply(x, \(xi) sum(xi, na.rm = TRUE))
/// purrr::map(x, ~ sum(.x))
/// sapply(x, function(xi) foo(xi) == 2)
/// ```
///
/// Use instead:
/// ```r
/// lapply(x, sum)
/// lapply(x, sum, na.rm = TRUE)
/// purrr::map(x, sum)
/// sapply(x, foo) == 2
/// ```
/// <!-- docs: end -->
pub fn unnecessary_lambda(
    ast: &RCall,
    fn_name: &str,
    allow_comparison: bool,
) -> anyhow::Result<Option<Diagnostic>> {
    let is_purrr = PURRR_MAPPERS.contains(&fn_name);
    if !is_purrr && !APPLY_FUNS.contains(&fn_name) {
        return Ok(None);
    }

    let outer_args: Vec<RArgument> = ast.arguments()?.items().iter().flatten().collect();
    // In `x |> lapply(f)`, `x` is an argument too.
    let n_other_args = outer_args.len() - 1 + usize::from(is_piped(ast));

    for arg in &outer_args {
        let Some(value) = arg.value() else {
            continue;
        };
        let diagnostic = if let Some(lambda) = value.as_r_function_definition() {
            check_lambda(lambda, fn_name, &outer_args, n_other_args, allow_comparison)
        } else if is_purrr && let Some(formula) = value.as_r_unary_expression() {
            check_formula(formula, fn_name, &outer_args)
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
    allow_comparison: bool,
) -> Option<Diagnostic> {
    let mut formals = vec![];
    for param in lambda.parameters().ok()?.items().iter() {
        let param = param.ok()?;
        if param.default().is_some() {
            return None;
        }
        formals.push(symbol_name(param.name().ok()?.syntax())?);
    }
    if formals.is_empty() {
        return None;
    }
    // Formals other than `...` must be supplied by the outer call.
    if formals.iter().filter(|f| *f != "...").count() > n_other_args {
        return None;
    }

    let body = unwrap_body(lambda.body().ok()?);

    if let Some(call) = body.as_r_call() {
        let (fun, fixable) = inner_call_target(call, &formals, &formals, outer_args)?;
        return Some(pass_directly_diagnostic(
            lambda.syntax(),
            &fun,
            fn_name,
            fixable,
        ));
    }

    if allow_comparison || !matches!(fn_name, "sapply" | "vapply") {
        return None;
    }
    let binary = body.as_r_binary_expression()?;
    let operator = binary.operator().ok()?;
    if !matches!(
        operator.kind(),
        RSyntaxKind::EQUAL2
            | RSyntaxKind::NOT_EQUAL
            | RSyntaxKind::LESS_THAN
            | RSyntaxKind::LESS_THAN_OR_EQUAL_TO
            | RSyntaxKind::GREATER_THAN
            | RSyntaxKind::GREATER_THAN_OR_EQUAL_TO
    ) {
        return None;
    }
    let (left, right) = (binary.left().ok()?, binary.right().ok()?);
    let (call, constant) = match (left.as_r_call(), right.as_r_call()) {
        (Some(call), None) if is_constant(&right) => (call, right.clone()),
        (None, Some(call)) if is_constant(&left) => (call, left.clone()),
        _ => return None,
    };
    let (fun, _) = inner_call_target(call, &formals, &formals, outer_args)?;

    let fun_value = if fn_name == "vapply" {
        ", FUN.VALUE = <intermediate>"
    } else {
        ""
    };
    Some(Diagnostic::new(
        ViolationData::new(
            Rule::UnnecessaryLambda,
            format!(
                "Comparing to a constant inside `{fn_name}()` doesn't benefit from vectorization."
            ),
            Some(format!(
                "Compare after calling `{fn_name}()`, e.g. `{fn_name}(x, {fun}{fun_value}) {} {}`.",
                operator.text_trimmed(),
                constant.syntax().text_trimmed()
            )),
        ),
        lambda.syntax().text_trimmed_range(),
        Fix::empty(),
    ))
}

/// purrr-style lambdas, e.g. `~ foo(.x)`.
fn check_formula(
    formula: &RUnaryExpression,
    fn_name: &str,
    outer_args: &[RArgument],
) -> Option<Diagnostic> {
    if formula.operator().ok()?.kind() != RSyntaxKind::TILDE {
        return None;
    }
    let body = unwrap_body(formula.argument().ok()?);
    let call = body.as_r_call()?;
    let first = call.arguments().ok()?.items().iter().next()?.ok()?;
    let first = symbol_name(first.value()?.syntax())?;
    if first != ".x" && first != "." {
        return None;
    }
    // All of these refer to the same element.
    let forbidden = [".x", ".", "..1"].map(String::from);
    let (fun, fixable) = inner_call_target(call, &[first], &forbidden, outer_args)?;
    Some(pass_directly_diagnostic(
        formula.syntax(),
        &fun,
        fn_name,
        fixable,
    ))
}

/// If `call` can replace the anonymous function, return the text of its
/// function and whether the anonymous function can be replaced by it as-is
/// (i.e. `call` has no arguments other than `formals`).
///
/// The leading arguments of `call` must be `formals` in order, the first one
/// passed positionally and the others passed the same way as in the outer
/// call (positionally or by matching name). The remaining arguments must not
/// refer to `forbidden` symbols nor contain calls, which would be evaluated
/// once instead of for each element.
fn inner_call_target(
    call: &RCall,
    formals: &[String],
    forbidden: &[String],
    outer_args: &[RArgument],
) -> Option<(String, bool)> {
    let function = call.function().ok()?;
    if function.as_r_identifier().is_none() && function.as_r_namespace_expression().is_none() {
        return None;
    }
    let fun = function.syntax().text_trimmed().to_string();
    if fun == "return" {
        return None;
    }

    let inner_args: Vec<RArgument> = call.arguments().ok()?.items().iter().flatten().collect();
    if inner_args.len() < formals.len() {
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

    for arg in &inner_args[formals.len()..] {
        let Some(value) = arg.value() else {
            continue;
        };
        let is_invalid = value.syntax().descendants().any(|node| {
            RCall::can_cast(node.kind())
                || symbol_name(&node).is_some_and(|name| forbidden.contains(&name))
        });
        if is_invalid {
            return None;
        }
    }

    Some((fun, inner_args.len() == formals.len()))
}

fn pass_directly_diagnostic(
    lambda: &RSyntaxNode,
    fun: &str,
    fn_name: &str,
    fixable: bool,
) -> Diagnostic {
    let range = lambda.text_trimmed_range();
    let suggestion = if fixable {
        format!("Pass `{fun}` directly to `{fn_name}()`.")
    } else {
        format!("Pass `{fun}` directly to `{fn_name}()`, followed by its other arguments.")
    };
    let fix = if fixable {
        Fix::new(range, fun.to_string(), node_contains_comments(lambda))
    } else {
        Fix::empty()
    };
    Diagnostic::new(
        ViolationData::new(
            Rule::UnnecessaryLambda,
            format!("Unnecessary anonymous function around `{fun}()`."),
            Some(suggestion),
        ),
        range,
        fix,
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

/// Numbers, strings, `TRUE`/`FALSE`, and complex numbers like `1 + 2i`.
fn is_constant(expr: &AnyRExpression) -> bool {
    match expr {
        AnyRExpression::AnyRValue(_)
        | AnyRExpression::RTrueExpression(_)
        | AnyRExpression::RFalseExpression(_) => true,
        AnyRExpression::RBinaryExpression(binary) => {
            binary
                .operator()
                .is_ok_and(|op| op.kind() == RSyntaxKind::PLUS)
                && binary.left().is_ok_and(|e| is_number(&e))
                && binary.right().is_ok_and(|e| is_number(&e))
        }
        _ => false,
    }
}

fn is_number(expr: &AnyRExpression) -> bool {
    matches!(
        expr,
        AnyRExpression::AnyRValue(
            AnyRValue::RDoubleValue(_) | AnyRValue::RIntegerValue(_) | AnyRValue::RComplexValue(_)
        )
    )
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
