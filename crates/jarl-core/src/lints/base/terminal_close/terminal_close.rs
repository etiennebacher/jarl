use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{StatementsExt, get_function_name};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

/// <!-- docs: start -->
/// Version added: 0.8.0
///
/// ## What it does
///
/// Checks for function definitions whose last call is `close(x)` or
/// `return(close(x))`.
///
/// ## Why is this bad?
///
/// `close()` is used to close a connection but might never trigger if there is
/// an error before it is called. It is safer to call `on.exit(close(x))` right
/// after the connection is created, so that it is closed however the function
/// exits.
///
/// This rule doesn't have an automatic fix because `on.exit()` must be placed
/// next to the place where the connection is opened.
///
/// ## Example
///
/// ```r
/// f <- function(path) {
///   conn <- file(path, open = "r")
///   out <- readLines(conn)
///   close(conn)
/// }
/// ```
///
/// Use instead:
/// ```r
/// f <- function(path) {
///   conn <- file(path, open = "r")
///   on.exit(close(conn))
///   readLines(conn)
/// }
/// ```
///
/// ## References
///
/// See `?on.exit` and `?close`.
/// <!-- docs: end -->
pub fn terminal_close(ast: &RFunctionDefinition) -> anyhow::Result<Option<Diagnostic>> {
    let body = ast.body()?;
    let Some(braced) = body.as_r_braced_statements() else {
        return Ok(None);
    };
    let Some(last) = braced.expressions().pop() else {
        return Ok(None);
    };
    let Some(call) = last.as_r_call() else {
        return Ok(None);
    };

    let close_call = match get_function_name(call.function()?).as_str() {
        "close" => call.clone(),
        "return" => {
            let args: Vec<_> = call.arguments()?.items().iter().flatten().collect();
            let inner = match args.as_slice() {
                [arg] => arg.value().and_then(|v| v.as_r_call().cloned()),
                _ => None,
            };
            match inner {
                Some(inner)
                    if inner
                        .function()
                        .is_ok_and(|f| get_function_name(f) == "close") =>
                {
                    inner
                }
                _ => return Ok(None),
            }
        }
        _ => return Ok(None),
    };

    // Only report connections created in the function itself: the closed
    // object must be a variable assigned in the function body (ignoring
    // nested functions).
    let Some(connection) = close_call
        .arguments()?
        .items()
        .iter()
        .flatten()
        .next()
        .and_then(|arg| arg.value())
        .and_then(|value| {
            value
                .as_r_identifier()
                .map(|id| id.syntax().text_trimmed().to_string())
        })
    else {
        return Ok(None);
    };

    let is_assigned = ast
        .syntax()
        .descendants()
        .filter_map(RBinaryExpression::cast)
        .filter(|binary| {
            binary
                .syntax()
                .ancestors()
                .find_map(RFunctionDefinition::cast)
                .is_some_and(|f| f.syntax() == ast.syntax())
        })
        .any(|binary| {
            let (Ok(left), Ok(op), Ok(right)) = (binary.left(), binary.operator(), binary.right())
            else {
                return false;
            };
            let target = match op.kind() {
                RSyntaxKind::ASSIGN | RSyntaxKind::SUPER_ASSIGN | RSyntaxKind::EQUAL => left,
                RSyntaxKind::ASSIGN_RIGHT | RSyntaxKind::SUPER_ASSIGN_RIGHT => right,
                _ => return false,
            };
            target.syntax().text_trimmed() == connection.as_str()
        });
    if !is_assigned {
        return Ok(None);
    }

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::TerminalClose,
            "`close()` is the last call of the function, so the connection stays open on errors or early returns.".to_string(),
            Some("Use `on.exit(close(x))` right after opening the connection.".to_string()),
        ),
        call.syntax().text_trimmed_range(),
        Fix::empty(),
    )))
}
