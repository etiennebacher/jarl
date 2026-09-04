use crate::diagnostic::*;
use crate::rule_set::Rule;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct AssignmentOnIfNoElse;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Flags assignments whose value is an `if` expression without a final `else`
/// branch, including an `else if` chain whose final `if` has no `else`.
///
/// ## Why is this bad?
///
/// When no branch is taken, an `if` expression without a final `else` evaluates
/// to `NULL`. Assigning that result can unexpectedly overwrite an existing
/// value.
///
/// ## Example
///
/// ```r
/// df <- if (condition) {
///   data.frame()
/// }
///
/// value <- if (a) {
///   1
/// } else if (b) {
///   2
/// }
/// ```
///
/// Use instead:
///
/// ```r
/// if (condition) {
///   df <- data.frame()
/// }
///
/// if (a) {
///   value <- 1
/// } else if (b) {
///   value <- 2
/// }
/// ```
///
/// If assigning a fallback value is appropriate, add a final `else` branch.
impl Violation for AssignmentOnIfNoElse {
    fn rule(&self) -> Rule {
        Rule::AssignmentOnIfNoElse
    }

    fn body(&self) -> String {
        "This assignment can overwrite the previous value with `NULL` when no `if` branch is taken."
            .to_string()
    }

    fn suggestion(&self) -> Option<String> {
        Some("Move the assignment into the `if` branches or add a final `else` branch.".to_string())
    }
}

pub fn assignment_on_if_no_else(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let operator = ast.operator()?;
    let value = match operator.kind() {
        RSyntaxKind::ASSIGN | RSyntaxKind::EQUAL | RSyntaxKind::SUPER_ASSIGN => ast.right()?,
        RSyntaxKind::ASSIGN_RIGHT | RSyntaxKind::SUPER_ASSIGN_RIGHT => ast.left()?,
        _ => return Ok(None),
    };

    let Some(if_statement) = as_if_statement(value)? else {
        return Ok(None);
    };

    if !has_no_final_else(&if_statement)? {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();
    Ok(Some(Diagnostic::new(
        AssignmentOnIfNoElse,
        range,
        Fix::empty(),
    )))
}

fn as_if_statement(expression: AnyRExpression) -> anyhow::Result<Option<RIfStatement>> {
    match expression {
        AnyRExpression::RIfStatement(if_statement) => Ok(Some(if_statement)),
        AnyRExpression::RParenthesizedExpression(parenthesized) => {
            as_if_statement(parenthesized.body()?)
        }
        _ => Ok(None),
    }
}

fn has_no_final_else(if_statement: &RIfStatement) -> anyhow::Result<bool> {
    let Some(else_clause) = if_statement.else_clause() else {
        return Ok(true);
    };

    let alternative = else_clause.alternative()?;
    let Some(next_if) = alternative.as_r_if_statement() else {
        return Ok(false);
    };

    has_no_final_else(next_if)
}
