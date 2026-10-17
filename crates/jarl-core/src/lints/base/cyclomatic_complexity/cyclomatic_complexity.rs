use air_r_syntax::*;
use biome_rowan::AstNode;

use super::flowgraph::{Lang, cyclocomp};
use crate::checker::Checker;
use crate::diagnostic::*;
use crate::rule_set::Rule;

/// <!-- docs: start -->
/// Version added: 0.7.0
///
/// ## What it does
///
/// Measures the cyclomatic complexity of each function and reports the ones above
/// `max-complexity` (20 by default).
///
/// When a function is above the maximum, the message lists the statements of its
/// body that add more than 20% of the score, as a starting point to split it.
///
/// ## Why is this bad?
///
/// The score counts the paths through the flow graph of the function.
/// A function with a high score can be hard to read, to test exhaustively, and
/// to change without breaking one of the paths nobody had in mind.
///
/// A high score might be due to a large number of `if` conditions, `for` loops,
/// or early `return()` calls, among other things. It generally indicates that
/// the function should be refactored, for instance by creating smaller
/// intermediate functions.
///
/// ## Details
///
/// While the way the score is computed tries to match the output of the
/// [`cyclocomp`](https://cran.r-project.org/package=cyclocomp) package (which is
/// also the output of `lintr::cyclocomp_linter()`), some differences exist:
///
/// * a chain of `&&` or `||` adds one path per operator. `cyclocomp` scores
///   `if (a || b || c || d) stop()` as 11, this rule as 5 (`stop()` can be
///   reached via paths, and we add one path if the condition is never true);
/// * `switch()` adds one path per arm, while `cyclocomp` ignores it;
/// * a function defined inside another one is scored on its own and counts as
///   a single node in the function that contains it, while `cyclocomp` adds all
///   of its paths to the ones of the outer function.
///
/// ## References
///
/// * [Cyclomatic complexity](https://en.wikipedia.org/wiki/Cyclomatic_complexity)
/// <!-- docs: end -->
pub fn cyclomatic_complexity(
    function: &RFunctionDefinition,
    checker: &Checker,
) -> anyhow::Result<Option<Diagnostic>> {
    let max_complexity = checker.rule_options.cyclomatic_complexity.max_complexity;

    let lowered = Lang::Function(Box::new(lower_result(function.body())));

    let Some(score) = cyclocomp(&lowered) else {
        return Ok(None);
    };
    if score <= max_complexity {
        return Ok(None);
    }

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::CyclomaticComplexity,
            format!(
                "This function has a cyclomatic complexity of {score}, above the maximum of {max_complexity}."
            ),
            Some(help_message(function, score)),
        ),
        // Point at the `function` keyword: the body can span hundreds of lines.
        function.name()?.text_trimmed_range(),
        Fix::empty(),
    )))
}

/// The statements of the body that, alone, account for more than this share of the
/// score are worth splitting first.
const MIN_SHARE_PERCENT: usize = 20;

fn help_message(function: &RFunctionDefinition, score: usize) -> String {
    let mut message = "Split this function into smaller ones.".to_string();

    let parts = biggest_parts(function, score);
    if !parts.is_empty() {
        let parts: Vec<String> = parts
            .iter()
            .map(|(text, share)| format!("`{text}` (+{share}, {}%)", share * 100 / score))
            .collect();
        message.push_str(&format!(" The largest parts are {}.", parts.join(", ")));
    }
    message
}

/// The statements of the body that account for more than `MIN_SHARE_PERCENT` of
/// the score, with the number of points they add, largest first. Each statement is
/// scored alone, in a function where it isn't the last expression: the points it
/// adds are the ones above the 1 of an empty body.
fn biggest_parts(function: &RFunctionDefinition, score: usize) -> Vec<(String, usize)> {
    let Ok(AnyRExpression::RBracedExpressions(braced)) = function.body() else {
        return vec![];
    };
    let statements: Vec<AnyRExpression> = braced.expressions().into_iter().collect();
    if statements.len() < 2 {
        return vec![];
    }

    let mut parts: Vec<(String, usize)> = statements
        .iter()
        .filter_map(|statement| {
            let alone = Lang::Function(Box::new(Lang::Call(vec![
                Lang::Atom,
                lower(statement),
                Lang::Atom,
            ])));
            let share = cyclocomp(&alone)?.checked_sub(1)?;
            (share * 100 > score * MIN_SHARE_PERCENT).then(|| (first_line(statement), share))
        })
        .collect();
    parts.sort_by_key(|(_, share)| std::cmp::Reverse(*share));
    parts
}

/// The first line of a statement, shortened.
fn first_line(statement: &AnyRExpression) -> String {
    const MAX_CHARS: usize = 40;

    let text = statement.syntax().text_trimmed().to_string();
    let line = text.lines().next().unwrap_or_default().trim();
    if line.chars().count() > MAX_CHARS {
        let shortened: String = line.chars().take(MAX_CHARS).collect();
        format!("{}...", shortened.trim_end())
    } else {
        line.to_string()
    }
}

/// Convert a syntax node to what R's parser would have produced for it.
fn lower(expression: &AnyRExpression) -> Lang {
    match expression {
        AnyRExpression::RBracedExpressions(braced) => {
            let mut elements = vec![Lang::Atom];
            elements.extend(braced.expressions().into_iter().map(|e| lower(&e)));
            Lang::Call(elements)
        }
        AnyRExpression::RParenthesizedExpression(parenthesized) => {
            match lower_result(parenthesized.body()) {
                // Parentheses around a condition don't hide its exits.
                condition @ Lang::AndOr { .. } => condition,
                body => Lang::Call(vec![Lang::Atom, body]),
            }
        }
        AnyRExpression::RUnaryExpression(unary) => {
            Lang::Call(vec![Lang::Atom, lower_result(unary.argument())])
        }
        AnyRExpression::RBinaryExpression(binary) => lower_binary(binary),
        // The selector is a symbol or a string.
        AnyRExpression::RExtractExpression(extract) => {
            Lang::Call(vec![Lang::Atom, lower_result(extract.left()), Lang::Atom])
        }
        AnyRExpression::RNamespaceExpression(_) => {
            Lang::Call(vec![Lang::Atom, Lang::Atom, Lang::Atom])
        }
        AnyRExpression::RCall(call) => {
            let Ok(function) = call.function() else {
                return Lang::Atom;
            };
            let Ok(arguments) = call.arguments() else {
                return Lang::Atom;
            };
            if is_identifier(&function, "switch")
                && let Some(switch) = lower_switch(&arguments.items())
            {
                return switch;
            }

            let mut elements = vec![lower(&function)];
            elements.extend(lower_arguments(&arguments.items()));
            call_node(&function, elements)
        }
        AnyRExpression::RSubset(subset) => {
            lower_subset(subset.function(), subset.arguments().map(|a| a.items()))
        }
        AnyRExpression::RSubset2(subset) => {
            lower_subset(subset.function(), subset.arguments().map(|a| a.items()))
        }
        AnyRExpression::RIfStatement(if_statement) => Lang::If {
            condition: Box::new(lower_result(if_statement.condition())),
            consequence: Box::new(lower_result(if_statement.consequence())),
            alternative: if_statement
                .else_clause()
                .map(|clause| Box::new(lower_result(clause.alternative()))),
        },
        AnyRExpression::RForStatement(for_statement) => Lang::For {
            sequence: Box::new(lower_result(for_statement.sequence())),
            body: Box::new(lower_result(for_statement.body())),
        },
        AnyRExpression::RWhileStatement(while_statement) => Lang::While {
            condition: Box::new(lower_result(while_statement.condition())),
            body: Box::new(lower_result(while_statement.body())),
        },
        AnyRExpression::RRepeatStatement(repeat) => {
            Lang::Repeat(Box::new(lower_result(repeat.body())))
        }
        AnyRExpression::RBreakExpression(_) => Lang::Break,
        AnyRExpression::RNextExpression(_) => Lang::Next,
        // Symbols, constants, and anything we can't make sense of. This includes
        // nested functions, which are scored on their own, see
        // `cyclomatic_complexity_function()`.
        _ => Lang::Atom,
    }
}

fn lower_result<E: Into<AnyRExpression>>(expression: biome_rowan::SyntaxResult<E>) -> Lang {
    match expression {
        Ok(expression) => lower(&expression.into()),
        Err(_) => Lang::Atom,
    }
}

fn lower_binary(binary: &RBinaryExpression) -> Lang {
    let (Ok(left), Ok(operator), Ok(right)) = (binary.left(), binary.operator(), binary.right())
    else {
        return Lang::Atom;
    };

    match operator.kind() {
        RSyntaxKind::AND2 | RSyntaxKind::OR2 => Lang::AndOr {
            is_and: operator.kind() == RSyntaxKind::AND2,
            left: Box::new(lower(&left)),
            right: Box::new(lower(&right)),
        },
        // R's parser turns `a -> b` into `` `<-`(b, a) ``.
        RSyntaxKind::ASSIGN_RIGHT | RSyntaxKind::SUPER_ASSIGN_RIGHT => {
            Lang::Call(vec![Lang::Atom, lower(&right), lower(&left)])
        }
        // ... and `lhs |> f(args)` into `f(lhs, args)`.
        RSyntaxKind::PIPE => match &right {
            AnyRExpression::RCall(call) => lower_piped_call(call, lower(&left)),
            _ => Lang::Call(vec![Lang::Atom, lower(&left), lower(&right)]),
        },
        _ => Lang::Call(vec![Lang::Atom, lower(&left), lower(&right)]),
    }
}

fn lower_piped_call(call: &RCall, lhs: Lang) -> Lang {
    let (Ok(function), Ok(arguments)) = (call.function(), call.arguments()) else {
        return Lang::Atom;
    };

    let mut elements = vec![lower(&function)];
    let mut lhs = Some(lhs);
    for argument in arguments.items().into_iter().flatten() {
        let is_placeholder = argument.name_clause().is_some()
            && matches!(
                argument.value(),
                Some(AnyRExpression::RIdentifier(ident)) if ident.syntax().text_trimmed() == "_"
            );
        match (is_placeholder, lhs.take()) {
            (true, Some(lhs)) => elements.push(lhs),
            (_, unused) => {
                lhs = unused;
                elements.push(argument.value().map_or(Lang::Atom, |v| lower(&v)));
            }
        }
    }
    if let Some(lhs) = lhs {
        elements.insert(1, lhs);
    }

    call_node(&function, elements)
}

fn lower_subset(
    function: biome_rowan::SyntaxResult<AnyRExpression>,
    items: biome_rowan::SyntaxResult<RArgumentList>,
) -> Lang {
    let (Ok(function), Ok(items)) = (function, items) else {
        return Lang::Atom;
    };
    let mut elements = vec![Lang::Atom, lower(&function)];
    let arguments = lower_arguments(&items);
    // `x[]` holds one empty argument in R.
    if arguments.is_empty() {
        elements.push(Lang::Atom);
    } else {
        elements.extend(arguments);
    }
    Lang::Call(elements)
}

fn lower_arguments(items: &RArgumentList) -> Vec<Lang> {
    items
        .into_iter()
        .filter_map(|argument| argument.ok())
        .map(|argument| argument.value().map_or(Lang::Atom, |v| lower(&v)))
        .collect()
}

/// A call to `callee`, which is a `return` if it is the bare symbol.
fn call_node(callee: &AnyRExpression, elements: Vec<Lang>) -> Lang {
    if is_identifier(callee, "return") {
        Lang::Return(elements)
    } else {
        Lang::Call(elements)
    }
}

/// cyclocomp only treats the bare symbol `return` as a return, not
/// `base::return`.
fn is_identifier(function: &AnyRExpression, name: &str) -> bool {
    matches!(
        function,
        AnyRExpression::RIdentifier(ident)
            if ident.syntax().text_trimmed().to_string().trim_matches('`') == name
    )
}

/// `switch(selector, a = x, b = , y)`: every arm after the selector is a path.
fn lower_switch(items: &RArgumentList) -> Option<Lang> {
    let mut arguments = items.into_iter().filter_map(|argument| argument.ok());
    let selector = lower(&arguments.next()?.value()?);

    let mut arms = vec![];
    let mut has_default = false;
    for argument in arguments {
        // An unnamed arm is the default.
        has_default |= argument.name_clause().is_none() && argument.value().is_some();
        arms.push(argument.value().map(|value| lower(&value)));
    }

    Some(Lang::Switch { selector: Box::new(selector), arms, has_default })
}
