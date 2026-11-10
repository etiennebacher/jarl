use crate::diagnostic::*;
use crate::lints::base::function_return::options::ReturnMode;
use crate::rule_set::Rule;
use crate::utils::{StatementsExt, get_function_name, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

/// Functions that are accepted as the last call of a function when
/// `mode = "explicit"`, since they either return or stop the execution.
const EXIT_FUNCTIONS: &[&str] = &[
    "return",
    "stop",
    ".Defunct",
    "abort",
    "cli_abort",
    "q",
    "quit",
    "invokeRestart",
    "tryInvokeRestart",
    "UseMethod",
    "NextMethod",
    "standardGeneric",
    "callNextMethod",
    ".C",
    ".Call",
    ".External",
    ".Fortran",
];

/// Functions that are only called for their side effects, so they are never
/// checked when `mode = "explicit"`.
const SKIPPED_FUNCTIONS: &[&str] = &[
    ".onLoad",
    ".onUnload",
    ".onAttach",
    ".onDetach",
    ".Last.lib",
    ".First",
    ".Last",
];

/// <!-- docs: start -->
/// Version added: 0.8.0
///
/// ## What it does
///
/// Checks that functions consistently use implicit or explicit `return()`. The
/// preferred mode can be configured, see below.
///
/// The returned values are searched in the last expression of the function
/// body, in the branches of terminal `if` statements and in the arms of
/// terminal `switch()` calls.
///
/// When `mode = "explicit"`, only functions whose body spans several lines
/// are checked. Functions ending with a call that stops the execution (such as
/// `stop()` or `UseMethod()`) and package hooks (such as `.onLoad()`) are not
/// reported.
///
/// ## Why is this bad?
///
/// In R, a function automatically returns the value of its last expression,
/// so a terminal `return()` call is redundant. At the same time, explicit
/// `return()` can clarify code, for instance across `if` branches. Using
/// explicit or implicit `return()` depends on personal preferences, but
/// mixing the two styles decreases readability and consistency.
///
/// This rule has an unsafe automatic fix because `return` may refer to a
/// function argument instead of `base::return()`, for instance in
/// `function(x, return) x`. Adding a call to `return()` in this function
/// would fail if the argument `return` is not provided.
///
/// The automatic fix is not available when the last expression is an assignment
/// or a loop, since wrapping it in `return()` would change its meaning or its
/// visibility.
///
/// ## Example
///
/// ```r
/// f <- function(x) {
///   if (x > 0) {
///     return("positive")
///   }
///   return("negative")
/// }
/// ```
///
/// If the preferred mode is "implicit", use instead:
/// ```r
/// f <- function(x) {
///   if (x > 0) {
///     return("positive")
///   }
///   "negative"
/// }
/// ```
///
/// ## References
///
/// - [Tidyverse style guide](https://style.tidyverse.org/functions.html#return)
/// <!-- docs: end -->
pub fn function_return(
    ast: &RFunctionDefinition,
    mode: ReturnMode,
) -> anyhow::Result<Vec<Diagnostic>> {
    let body = ast.body()?;

    if mode == ReturnMode::Explicit {
        // Like inline functions (`function(x) x + 1`), braced bodies whose
        // last expression is on the same line as `{` are not checked.
        let Some(last) = body
            .as_r_braced_statements()
            .and_then(|braced| braced.expressions().pop())
        else {
            return Ok(vec![]);
        };
        let on_new_line = last
            .syntax()
            .first_token()
            .is_some_and(|token| token.has_leading_newline());
        if !on_new_line || is_skipped_function(ast) {
            return Ok(vec![]);
        }
    }

    let mut terminals = Vec::new();
    collect_terminal_expressions(body, &mut terminals);

    let diagnostics = terminals
        .iter()
        .filter_map(|expr| match mode {
            ReturnMode::Implicit => implicit_return_diagnostic(expr),
            ReturnMode::Explicit => explicit_return_diagnostic(expr),
        })
        .collect();

    Ok(diagnostics)
}

/// Collect the expressions whose value can be returned by the function.
fn collect_terminal_expressions(expr: AnyRExpression, out: &mut Vec<AnyRExpression>) {
    match &expr {
        AnyRExpression::RBracedStatements(braced) => match braced.expressions().pop() {
            Some(last) => collect_terminal_expressions(last, out),
            None => out.push(expr),
        },
        AnyRExpression::RIfStatement(if_stmt) => {
            if let Ok(consequence) = if_stmt.consequence() {
                collect_terminal_expressions(consequence, out);
            }
            if let Some(alternative) = if_stmt.else_clause().and_then(|x| x.alternative().ok()) {
                collect_terminal_expressions(alternative, out);
            }
        }
        AnyRExpression::RCall(call)
            if call
                .function()
                .is_ok_and(|f| get_function_name(f) == "switch") =>
        {
            let Ok(args) = call.arguments() else {
                return;
            };
            // The first argument is the value to switch on.
            for value in args
                .items()
                .iter()
                .skip(1)
                .filter_map(|arg| arg.ok()?.value())
            {
                collect_terminal_expressions(value, out);
            }
        }
        _ => out.push(expr),
    }
}

fn implicit_return_diagnostic(expr: &AnyRExpression) -> Option<Diagnostic> {
    let call = expr.as_r_call()?;
    if get_function_name(call.function().ok()?) != "return" {
        return None;
    }

    let args: Vec<RArgument> = call.arguments().ok()?.items().iter().flatten().collect();
    let replacement = match args.as_slice() {
        [] => Some("NULL".to_string()),
        [arg] if arg.name_clause().is_none() => arg
            .value()
            .filter(|value| !changes_when_unwrapped(value))
            .map(|value| value.syntax().text_trimmed().to_string()),
        _ => None,
    };

    let range = call.syntax().text_trimmed_range();
    let fix = match replacement {
        Some(replacement) => Fix::new(range, replacement, node_contains_comments(call.syntax())),
        None => Fix::empty(),
    };

    Some(Diagnostic::new(
        ViolationData::new(
            Rule::FunctionReturn,
            "Explicit `return()` is not needed at the end of a function.".to_string(),
            Some("Remove the `return()` call and keep its value.".to_string()),
        ),
        range,
        fix,
    ))
}

fn explicit_return_diagnostic(expr: &AnyRExpression) -> Option<Diagnostic> {
    if is_exit_call(expr) {
        return None;
    }

    let range = expr.syntax().text_trimmed_range();
    let fix = if changes_when_wrapped(expr) {
        Fix::empty()
    } else {
        Fix::from_edits(
            vec![
                Edit::insertion(range.start(), "return(".to_string()),
                Edit::insertion(range.end(), ")".to_string()),
            ],
            false,
        )
    };

    Some(Diagnostic::new(
        ViolationData::new(
            Rule::FunctionReturn,
            "Missing explicit `return()` at the end of a function.".to_string(),
            Some("Wrap the returned value in `return()`.".to_string()),
        ),
        range,
        fix,
    ))
}

/// Whether `expr` is a call to one of [EXIT_FUNCTIONS], possibly at the end
/// of a magrittr pipe chain (`x %>% return()`).
fn is_exit_call(expr: &AnyRExpression) -> bool {
    let expr = match expr.as_r_binary_expression() {
        Some(binary)
            if binary.operator().is_ok_and(|op| {
                op.kind() == RSyntaxKind::SPECIAL && op.text_trimmed() == "%>%"
            }) =>
        {
            match binary.right() {
                Ok(right) => right,
                Err(_) => return false,
            }
        }
        _ => expr.clone(),
    };

    expr.as_r_call()
        .and_then(|call| call.function().ok())
        .is_some_and(|f| EXIT_FUNCTIONS.contains(&get_function_name(f).as_str()))
}

/// Whether wrapping `expr` in `return()` would change the returned value or
/// its visibility: `return(x = 1)` passes a named argument, and loops return
/// an invisible `NULL` that `return()` makes visible.
fn changes_when_wrapped(expr: &AnyRExpression) -> bool {
    match expr {
        AnyRExpression::RBinaryExpression(binary) => binary.operator().is_ok_and(|op| {
            matches!(
                op.kind(),
                RSyntaxKind::ASSIGN
                    | RSyntaxKind::SUPER_ASSIGN
                    | RSyntaxKind::ASSIGN_RIGHT
                    | RSyntaxKind::SUPER_ASSIGN_RIGHT
                    | RSyntaxKind::EQUAL
            )
        }),
        AnyRExpression::RBracedStatements(braced) => braced.expressions().is_empty(),
        AnyRExpression::RForStatement(_)
        | AnyRExpression::RWhileStatement(_)
        | AnyRExpression::RRepeatStatement(_) => true,
        _ => false,
    }
}

/// Whether removing the `return()` around `value` would change the returned
/// value or its visibility: loops and `if` without `else` return an invisible
/// `NULL`, and the latter could also capture a following `else`.
fn changes_when_unwrapped(value: &AnyRExpression) -> bool {
    match value {
        AnyRExpression::RIfStatement(if_stmt) => if_stmt.else_clause().is_none(),
        AnyRExpression::RForStatement(_)
        | AnyRExpression::RWhileStatement(_)
        | AnyRExpression::RRepeatStatement(_) => true,
        _ => false,
    }
}

/// Whether the function is assigned to one of [SKIPPED_FUNCTIONS].
fn is_skipped_function(ast: &RFunctionDefinition) -> bool {
    let Some(binary) = ast.syntax().parent().and_then(RBinaryExpression::cast) else {
        return false;
    };
    let (Ok(left), Ok(operator), Ok(right)) = (binary.left(), binary.operator(), binary.right())
    else {
        return false;
    };
    let name = match operator.kind() {
        RSyntaxKind::ASSIGN | RSyntaxKind::SUPER_ASSIGN | RSyntaxKind::EQUAL => left,
        RSyntaxKind::ASSIGN_RIGHT | RSyntaxKind::SUPER_ASSIGN_RIGHT => right,
        _ => return false,
    };
    SKIPPED_FUNCTIONS.contains(&get_function_name(name).as_str())
}
