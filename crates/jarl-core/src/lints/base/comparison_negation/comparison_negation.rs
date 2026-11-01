use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::node_contains_comments;
use air_r_syntax::*;
use biome_rowan::AstNode;

/// <!-- docs: start -->
/// Version added: 0.0.23
///
/// ## What it does
///
/// Checks for negated comparisons, such as `!(x < y)` or `!x < y`.
///
/// ## Why is this bad?
///
/// This pattern may be hard to read and could be simplified by removing the `!`
/// operator and inverting the operator (e.g. `<` would become `>=`).
///
/// The fix preserves parentheses where needed to keep operator precedence.
///
/// ## Example
///
/// ```r
/// !(x < y + 1)
/// !(x == y + 1)
/// ```
///
/// Use instead:
/// ```r
/// x >= y + 1
/// x != y + 1
/// ```
/// <!-- docs: end -->
pub fn comparison_negation(ast: &RUnaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let operator = ast.operator()?;

    if operator.kind() != RSyntaxKind::BANG {
        return Ok(None);
    }

    // A bang directly inside another bang may be part of tidy evaluation's
    // !! or !!!. Leave the entire injection expression alone.
    if let Some(parent) = ast.syntax().parent().and_then(RUnaryExpression::cast)
        && parent.operator()?.kind() == RSyntaxKind::BANG
    {
        return Ok(None);
    }

    let mut argument = ast.argument()?;
    while let AnyRExpression::RParenthesizedExpression(paren) = argument {
        argument = paren.body()?;
    }

    let comparison = unwrap_or_return_none!(argument.as_r_binary_expression());
    let replacement_operator = match comparison.operator()?.kind() {
        RSyntaxKind::GREATER_THAN => "<=",
        RSyntaxKind::GREATER_THAN_OR_EQUAL_TO => "<",
        RSyntaxKind::LESS_THAN => ">=",
        RSyntaxKind::LESS_THAN_OR_EQUAL_TO => ">",
        RSyntaxKind::EQUAL2 => "!=",
        RSyntaxKind::NOT_EQUAL => "==",
        _ => return Ok(None),
    };

    let mut replacement = format!(
        "{} {} {}",
        comparison.left()?.to_trimmed_text(),
        replacement_operator,
        comparison.right()?.to_trimmed_text()
    );

    // Comparisons bind more tightly than assignment, logical, and formula
    // operators. Group the replacement in other operand positions; an existing
    // parenthesized expression already supplies the necessary grouping, including
    // when the negation is called, indexed, or used with `$` or `@`.
    let needs_parentheses = match ast.syntax().parent().and_then(AnyRExpression::cast) {
        Some(AnyRExpression::RBinaryExpression(binary)) => !matches!(
            binary.operator()?.text_trimmed(),
            "<-" | "<<-" | "->" | "->>" | "=" | "&" | "&&" | "|" | "||" | "~"
        ),
        Some(AnyRExpression::RUnaryExpression(_)) => true,
        _ => false,
    };
    if needs_parentheses {
        replacement = format!("({replacement})");
    }

    let range = ast.syntax().text_trimmed_range();
    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::ComparisonNegation,
            "This negated comparison can be simplified.".to_string(),
            Some(format!("Use `{replacement}` instead.")),
        ),
        range,
        Fix::new(range, replacement, node_contains_comments(ast.syntax())),
    )))
}
