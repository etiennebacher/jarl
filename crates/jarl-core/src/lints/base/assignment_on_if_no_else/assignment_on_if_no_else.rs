use crate::diagnostic::*;
use crate::rule_set::Rule;
use air_r_syntax::*;
use biome_rowan::AstNode;
use jarl_semantic::{SemanticInfo, assignment_lhs_name};

pub struct AssignmentOnIfNoElse;

/// <!-- docs: start -->
/// Version added: 0.8.0
///
/// ## What it does
///
/// Reports assignments whose value is an `if` expression without a final `else`
/// branch, when the assignment overwrites an earlier value.
///
/// ## Why is this bad?
///
/// When no branch is taken, an `if` expression without a final `else` evaluates
/// to `NULL`. This can be unexpected since one could believe that if no branch
/// is taken then the previous value is kept.
///
/// ## Example
///
/// ```r
/// df <- mtcars
/// # If `condition` is `FALSE`, `df` becomes `NULL` instead of
/// # keeping its original value (`mtcars`).
/// df <- if (condition) {
///   mtcars[1:5, ]
/// }
///
/// value <- 1
/// # If neither `a` nor `b` is `TRUE`, `value` becomes `NULL`
/// # instead of keeping its original value (`1`).
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
///   df <- mtcars[1:5, ]
/// }
///
/// if (a) {
///   value <- 1
/// } else if (b) {
///   value <- 2
/// }
/// ```
///
/// If `NULL` is the intended fallback, add a final `else` branch that says so.
/// <!-- docs: end -->
impl Violation for AssignmentOnIfNoElse {
    fn rule(&self) -> Rule {
        Rule::AssignmentOnIfNoElse
    }

    fn body(&self) -> String {
        "This assignment can overwrite the previous value with `NULL` when no `if` branch is taken."
            .to_string()
    }

    fn suggestion(&self) -> Option<String> {
        Some(
            "Add a final `else` branch if `NULL` is the intended fallback, or move the assignment into the `if` branches."
                .to_string(),
        )
    }
}

pub fn assignment_on_if_no_else(
    ast: &RBinaryExpression,
    semantic: &SemanticInfo<'_>,
) -> anyhow::Result<Option<Diagnostic>> {
    // Incomplete code (`f <-`) has a missing operand: nothing to lint.
    let Ok(operator) = ast.operator() else {
        return Ok(None);
    };
    let value = match operator.kind() {
        RSyntaxKind::ASSIGN | RSyntaxKind::EQUAL | RSyntaxKind::SUPER_ASSIGN => ast.right(),
        RSyntaxKind::ASSIGN_RIGHT | RSyntaxKind::SUPER_ASSIGN_RIGHT => ast.left(),
        _ => return Ok(None),
    };
    let Ok(value) = value else {
        return Ok(None);
    };

    let Some(if_statement) = as_if_statement(value) else {
        return Ok(None);
    };

    if !has_no_final_else(&if_statement) {
        return Ok(None);
    }

    let Some(name) = assignment_lhs_name(ast.syntax()) else {
        return Ok(None);
    };

    let range = ast.syntax().text_trimmed_range();
    let Some(previous) = semantic.last_assignment_before(&name, range) else {
        return Ok(None);
    };

    Ok(Some(
        Diagnostic::new(AssignmentOnIfNoElse, range, Fix::empty()).with_labels(vec![Label::new(
            previous,
            format!("`{name}` was last assigned here"),
        )]),
    ))
}

fn as_if_statement(expression: AnyRExpression) -> Option<RIfStatement> {
    match expression {
        AnyRExpression::RIfStatement(if_statement) => Some(if_statement),
        AnyRExpression::RParenthesizedExpression(parenthesized) => {
            as_if_statement(parenthesized.body().ok()?)
        }
        _ => None,
    }
}

fn has_no_final_else(if_statement: &RIfStatement) -> bool {
    let Some(else_clause) = if_statement.else_clause() else {
        return true;
    };

    let Ok(alternative) = else_clause.alternative() else {
        return false;
    };
    let Some(next_if) = alternative.as_r_if_statement() else {
        return false;
    };

    has_no_final_else(next_if)
}
