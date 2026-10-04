use std::collections::HashSet;

use air_r_syntax::*;
use biome_rowan::{AstNode, TextRange, WalkEvent};

use crate::checker::Checker;
use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{argument_name, get_function_name};

/// <!-- docs: start -->
/// Version added: 0.7.0
///
/// ## What it does
///
/// Measures the cyclomatic complexity of each function, and of the top-level
/// code of a file, and reports the ones above `max-complexity` (15 by default).
///
/// The score starts at 1 and gains a point for every place the code can take a
/// different path:
///
/// * each `if` (an `else if` is an `if` too, but a bare `else` adds nothing);
/// * each `for`, `while` and `repeat` loop;
/// * each `&&` and `||`. The vectorized `&` and `|` always evaluate both sides,
///   so they add nothing;
/// * each `switch()` arm after the first;
/// * each handler of `tryCatch()` and `withCallingHandlers()`, and each `try()`;
/// * each `return()` or `stop()` that isn't the value the function ends on.
///
/// Nested functions are scored on their own and don't count towards the
/// function that contains them.
///
/// ## Why is this bad?
///
/// The score counts the paths through the code, which is also the number of
/// tests needed to cover it. A function with a high score is hard to read, hard
/// to test exhaustively, and hard to change without breaking one of the paths
/// nobody had in mind.
///
/// ## Example
///
/// ```r
/// summarize <- function(x, na.rm, kind) {
///   if (!is.numeric(x)) stop("`x` must be numeric.")
///   if (na.rm && anyNA(x)) x <- x[!is.na(x)]
///   switch(kind,
///     mean = mean(x),
///     median = median(x),
///     stop("Unknown `kind`.")
///   )
/// }
/// ```
///
/// Use instead:
/// ```r
/// check_input <- function(x) {
///   if (!is.numeric(x)) stop("`x` must be numeric.")
/// }
///
/// summarize <- function(x, na.rm, kind) {
///   check_input(x)
///   if (na.rm && anyNA(x)) x <- x[!is.na(x)]
///   summary_fun(kind)(x)
/// }
/// ```
///
/// ## Options
///
/// * `max-complexity`
/// <!-- docs: end -->
pub fn cyclomatic_complexity(
    ast: &RFunctionDefinition,
    checker: &Checker,
) -> anyhow::Result<Option<Diagnostic>> {
    let options = &checker.rule_options.cyclomatic_complexity;
    let body = ast.body()?;

    let mut tails = HashSet::new();
    collect_tail_positions(&body, &mut tails);

    let score = compute_complexity(
        std::slice::from_ref(body.syntax()),
        &options.stopping_functions,
        &tails,
    );
    if score <= options.max_complexity {
        return Ok(None);
    }

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::CyclomaticComplexity,
            format!(
                "This function has a cyclomatic complexity of {score}, above the maximum of {}.",
                options.max_complexity
            ),
            Some("Split this function into smaller ones.".to_string()),
        ),
        // Point at the `function` keyword: the body can span hundreds of lines.
        ast.name()?.text_trimmed_range(),
        Fix::empty(),
    )))
}

/// Measure the top-level code of a file, i.e. everything outside of a function
/// definition, as a single unit.
pub fn cyclomatic_complexity_top_level(
    expressions: &[RSyntaxNode],
    checker: &Checker,
) -> anyhow::Result<Option<Diagnostic>> {
    let options = &checker.rule_options.cyclomatic_complexity;
    let Some(first) = expressions.first() else {
        return Ok(None);
    };

    let mut tails = HashSet::new();
    if let Some(last) = expressions.last().and_then(AnyRExpression::cast_ref) {
        collect_tail_positions(&last, &mut tails);
    }

    let score = compute_complexity(expressions, &options.stopping_functions, &tails);
    if score <= options.max_complexity {
        return Ok(None);
    }

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::CyclomaticComplexity,
            format!(
                "The top-level code of this file has a cyclomatic complexity of {score}, \
                 above the maximum of {}.",
                options.max_complexity
            ),
            Some("Move this logic into functions.".to_string()),
        ),
        first.text_trimmed_range(),
        Fix::empty(),
    )))
}

/// Walk `roots` and add up the branch points they contain.
///
/// Function definitions are skipped: each one is measured on its own, so the
/// paths inside a closure belong to the closure and not to whatever surrounds
/// it.
fn compute_complexity(
    roots: &[RSyntaxNode],
    stopping: &HashSet<String>,
    tails: &HashSet<TextRange>,
) -> usize {
    // A branchless body still has one path through it.
    let mut score = 1;

    for root in roots {
        let mut preorder = root.preorder();
        while let Some(event) = preorder.next() {
            let WalkEvent::Enter(node) = event else {
                continue;
            };

            if node.kind() == RSyntaxKind::R_FUNCTION_DEFINITION {
                preorder.skip_subtree();
                continue;
            }

            score += node_cost(&node, stopping, tails);
        }
    }

    score
}

fn node_cost(node: &RSyntaxNode, stopping: &HashSet<String>, tails: &HashSet<TextRange>) -> usize {
    match node.kind() {
        RSyntaxKind::R_IF_STATEMENT
        | RSyntaxKind::R_FOR_STATEMENT
        | RSyntaxKind::R_WHILE_STATEMENT
        | RSyntaxKind::R_REPEAT_STATEMENT => 1,
        RSyntaxKind::R_BINARY_EXPRESSION => {
            let Some(expression) = RBinaryExpression::cast_ref(node) else {
                return 0;
            };
            let Ok(operator) = expression.operator() else {
                return 0;
            };
            // Only `&&` and `||` short-circuit; `&` and `|` are vectorized and
            // always evaluate both sides, so they don't create a path.
            usize::from(matches!(
                operator.kind(),
                RSyntaxKind::AND2 | RSyntaxKind::OR2
            ))
        }
        RSyntaxKind::R_CALL => {
            let Some(call) = RCall::cast_ref(node) else {
                return 0;
            };
            call_cost(&call, stopping, tails)
        }
        _ => 0,
    }
}

fn call_cost(call: &RCall, stopping: &HashSet<String>, tails: &HashSet<TextRange>) -> usize {
    let Ok(function) = call.function() else {
        return 0;
    };
    let Ok(arguments) = call.arguments() else {
        return 0;
    };
    let name = get_function_name(function);

    match name.as_str() {
        // The first argument is the value being matched, so a `switch()` only
        // starts branching at its second arm.
        "switch" => arguments.items().into_iter().count().saturating_sub(2),
        // Every handler is another way out of the guarded expression. `expr` is
        // that expression and `finally` runs on all paths, so neither counts.
        "tryCatch" | "withCallingHandlers" => arguments
            .items()
            .into_iter()
            .filter_map(|argument| argument_name(&argument.ok()?))
            .filter(|name| name != "expr" && name != "finally")
            .count(),
        "try" => 1,
        "return" => usize::from(!is_tail(call, tails)),
        _ => usize::from(stopping.contains(&name) && !is_tail(call, tails)),
    }
}

/// An exit in tail position is how the code normally ends, so it doesn't add a
/// path. Only the ones jumping out from somewhere else do.
fn is_tail(call: &RCall, tails: &HashSet<TextRange>) -> bool {
    tails.contains(&call.syntax().text_trimmed_range())
}

/// Collect the expressions that give the code its value: the expression itself,
/// the last one of a `{` block, and both branches of a trailing `if`.
fn collect_tail_positions(expression: &AnyRExpression, tails: &mut HashSet<TextRange>) {
    tails.insert(expression.syntax().text_trimmed_range());

    match expression {
        AnyRExpression::RBracedExpressions(braced) => {
            if let Some(last) = braced.expressions().into_iter().last() {
                collect_tail_positions(&last, tails);
            }
        }
        AnyRExpression::RParenthesizedExpression(parenthesized) => {
            if let Ok(inner) = parenthesized.body() {
                collect_tail_positions(&inner, tails);
            }
        }
        AnyRExpression::RIfStatement(if_statement) => {
            if let Ok(consequence) = if_statement.consequence() {
                collect_tail_positions(&consequence, tails);
            }
            if let Some(else_clause) = if_statement.else_clause()
                && let Ok(alternative) = else_clause.alternative()
            {
                collect_tail_positions(&alternative, tails);
            }
        }
        _ => {}
    }
}
