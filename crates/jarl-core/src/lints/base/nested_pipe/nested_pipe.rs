use crate::checker::Checker;
use crate::diagnostic::*;
use crate::utils::get_function_name;
use air_r_syntax::*;
use biome_rowan::{AstNode, SyntaxResult};

pub struct NestedPipe;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Reports pipes (`%>%` or `|>`) that are nested inside another function call,
/// e.g. `print(x %>% foo())`.
///
/// ## Why is this bad?
///
/// Nesting a pipe inside another call hides the data flow and makes the code
/// harder to read. Extracting the pipe into its own statement keeps each step
/// on its own line.
///
/// `try()`, `tryCatch()`, and `withCallingHandlers()` are automatically skipped.
/// The list of skipped functions can be customized with [rule-specific arguments](https://jarl.etiennebacher.com/reference/config-file#rule-specific-arguments)
/// in `jarl.toml`.
///
/// ## Example
///
/// ```r
/// print(x %>% foo() %>% bar())
/// ```
///
/// Use instead:
/// ```r
/// out <- x %>%
///   foo() %>%
///   bar()
///
/// print(out)
/// ```
impl Violation for NestedPipe {
    fn name(&self) -> String {
        "nested_pipe".to_string()
    }
    fn body(&self) -> String {
        "Don't nest pipes inside other calls.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Extract the pipe into its own statement and pass the result to the call.".to_string())
    }
}

pub fn nested_pipe(
    ast: &RBinaryExpression,
    checker: &Checker,
) -> anyhow::Result<Option<Diagnostic>> {
    let operator = ast.operator()?;
    if !is_pipe(&operator) {
        return Ok(None);
    }

    // A pipe chain such as `a %>% b() %>% c()` parses as nested binary
    // expressions where each inner pipe is the left-hand side of the next one.
    // Only act on the outermost pipe of the chain so the whole chain produces a
    // single diagnostic.
    if let Some(parent) = ast.syntax().parent()
        && let Some(parent_binary) = RBinaryExpression::cast(parent)
        && let Ok(parent_op) = parent_binary.operator()
        && is_pipe(&parent_op)
    {
        return Ok(None);
    }

    // Find the enclosing function-call argument, if any. A braced block (`{}`)
    // introduces a new statement context, so a pipe inside it is not considered
    // "nested in a call" (e.g. a function body or `local({ ... })`).
    let mut enclosing_arg = None;
    for ancestor in ast.syntax().ancestors() {
        if RBracedExpressions::can_cast(ancestor.kind()) {
            return Ok(None);
        }
        if let Some(arg) = RArgument::cast(ancestor) {
            enclosing_arg = Some(arg);
            break;
        }
    }
    let Some(enclosing_arg) = enclosing_arg else {
        // Top level, assignment right-hand side, etc.: not nested in a call.
        return Ok(None);
    };

    let Some(call) = enclosing_arg.syntax().ancestors().find_map(RCall::cast) else {
        return Ok(None);
    };
    let function_name = get_function_name(call.function()?);

    if function_name == "switch" {
        // The first argument of `switch()` is the value being switched on (an
        // input position) and is linted; every other argument is an output
        // position and is allowed.
        if !is_first_argument(&call, &enclosing_arg) {
            return Ok(None);
        }
    } else {
        let skipped = &checker.rule_options.nested_pipe.skipped_functions;
        if skipped.contains(&function_name) {
            return Ok(None);
        }
    }

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(NestedPipe, range, Fix::empty());
    Ok(Some(diagnostic))
}

fn is_pipe(operator: &RSyntaxToken) -> bool {
    operator.kind() == RSyntaxKind::PIPE
        || (operator.kind() == RSyntaxKind::SPECIAL && operator.text_trimmed() == "%>%")
}

/// Whether `arg` is the first argument of `call`.
fn is_first_argument(call: &RCall, arg: &RArgument) -> bool {
    call.arguments()
        .ok()
        .and_then(|args| args.items().into_iter().next())
        .and_then(SyntaxResult::ok)
        .is_some_and(|first| first.syntax() == arg.syntax())
}
