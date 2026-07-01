use crate::diagnostic::{Diagnostic, Fix, ViolationData};
use crate::utils::node_contains_comments;
use air_r_syntax::{
    AnyRExpression, RBinaryExpression, RParenthesizedExpression, RSyntaxKind, RUnaryExpression,
};
use biome_rowan::AstNode;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for expressions wrapped in multiple pairs of parentheses.
///
/// ## Why is this bad?
///
/// Repeated parentheses do not change the meaning of the expression and can make
/// the code harder to read.
///
/// ## Example
///
/// ```r
/// ((x + 1))
/// ```
///
/// Use instead:
///
/// ```r
/// x + 1
/// ```
pub fn unnecessary_parentheses(
    ast: &RParenthesizedExpression,
) -> anyhow::Result<Option<Diagnostic>> {
    if ast
        .syntax()
        .parent()
        .and_then(RParenthesizedExpression::cast)
        .is_some()
    {
        return Ok(None);
    }

    let mut count = 1;
    let mut current = ast.body()?;

    // we count the number of nested unnecessary parentheses
    while let Some(inner) = current.as_r_parenthesized_expression() {
        count += 1;
        current = inner.body()?;
    }

    if count == 1 {
        return Ok(None);
    }

    let keep_outer = needs_outer_parens(ast, &current)?;
    let removable_count = count - usize::from(keep_outer);

    let (body, suggestion) = if removable_count == 1 {
        (
            "This expression contains an unnecessary pair of parentheses.".to_string(),
            "Remove the unnecessary pair of parentheses.".to_string(),
        )
    } else {
        (
            format!("This expression contains {removable_count} unnecessary pairs of parentheses."),
            format!("Remove {removable_count} pairs of parentheses."),
        )
    };

    let range = ast.syntax().text_trimmed_range();

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            "unnecessary_parentheses".to_string(),
            body,
            Some(suggestion),
        ),
        range,
        Fix {
            content: if keep_outer {
                format!("({})", current.to_trimmed_string())
            } else {
                current.to_trimmed_string()
            },
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    )))
}

// Returns true if the outermost parenthesis pair must be kept to preserve semantics.
fn needs_outer_parens(
    ast: &RParenthesizedExpression,
    current: &AnyRExpression,
) -> anyhow::Result<bool> {
    let Some(parent) = ast.syntax().parent() else {
        return Ok(false);
    };

    if let Some(parent_binary) = RBinaryExpression::cast(parent.clone()) {
        return needs_parentheses_in_binary_parent(ast, current, &parent_binary);
    }

    if RUnaryExpression::cast(parent).is_some() && current.as_r_binary_expression().is_some() {
        return Ok(true);
    }

    Ok(false)
}

// Returns true if `ast` needs parentheses given that its parent is a binary expression.
fn needs_parentheses_in_binary_parent(
    ast: &RParenthesizedExpression,
    current: &AnyRExpression,
    parent: &RBinaryExpression,
) -> anyhow::Result<bool> {
    let parent_operator = parent.operator()?.kind();

    if current.as_r_unary_expression().is_some() {
        return Ok(matches!(
            parent_operator,
            RSyntaxKind::EXPONENTIATE | RSyntaxKind::EXPONENTIATE2
        ));
    }

    let Some(current_binary) = current.as_r_binary_expression() else {
        return Ok(false);
    };

    let Some(parent_precedence) = binary_precedence(parent_operator) else {
        return Ok(true);
    };

    let current_operator = current_binary.operator()?.kind();
    let Some(current_precedence) = binary_precedence(current_operator) else {
        return Ok(true);
    };

    match current_precedence.cmp(&parent_precedence) {
        std::cmp::Ordering::Less => return Ok(true),
        std::cmp::Ordering::Greater => return Ok(false),
        std::cmp::Ordering::Equal => {}
    }

    let side = child_side(ast, parent)?;

    Ok(match side {
        Some(ChildSide::Left) => !matches!(
            parent_operator,
            RSyntaxKind::PLUS | RSyntaxKind::MINUS | RSyntaxKind::MULTIPLY | RSyntaxKind::DIVIDE
        ),
        Some(ChildSide::Right) => {
            !(parent_operator == current_operator
                && matches!(
                    parent_operator,
                    RSyntaxKind::EXPONENTIATE | RSyntaxKind::EXPONENTIATE2
                ))
        }
        None => true,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildSide {
    Left,
    Right,
}

// Determines whether `ast` is the left or right operand of `parent`.
fn child_side(
    ast: &RParenthesizedExpression,
    parent: &RBinaryExpression,
) -> anyhow::Result<Option<ChildSide>> {
    let range = ast.syntax().text_trimmed_range();

    if parent.left()?.syntax().text_trimmed_range() == range {
        return Ok(Some(ChildSide::Left));
    }

    if parent.right()?.syntax().text_trimmed_range() == range {
        return Ok(Some(ChildSide::Right));
    }

    Ok(None)
}

// Returns the precedence level of a binary operator (higher = tighter binding), or None for unknown operators.
fn binary_precedence(operator: RSyntaxKind) -> Option<u8> {
    Some(match operator {
        RSyntaxKind::ASSIGN
        | RSyntaxKind::SUPER_ASSIGN
        | RSyntaxKind::ASSIGN_RIGHT
        | RSyntaxKind::SUPER_ASSIGN_RIGHT
        | RSyntaxKind::WALRUS => 1,
        RSyntaxKind::TILDE => 2,
        RSyntaxKind::PIPE => 3,
        RSyntaxKind::OR | RSyntaxKind::OR2 => 4,
        RSyntaxKind::AND | RSyntaxKind::AND2 => 5,
        RSyntaxKind::LESS_THAN
        | RSyntaxKind::LESS_THAN_OR_EQUAL_TO
        | RSyntaxKind::GREATER_THAN
        | RSyntaxKind::GREATER_THAN_OR_EQUAL_TO
        | RSyntaxKind::EQUAL2
        | RSyntaxKind::NOT_EQUAL => 6,
        RSyntaxKind::PLUS | RSyntaxKind::MINUS => 7,
        RSyntaxKind::MULTIPLY | RSyntaxKind::DIVIDE => 8,
        RSyntaxKind::SPECIAL => 9,
        RSyntaxKind::COLON => 10,
        RSyntaxKind::EXPONENTIATE | RSyntaxKind::EXPONENTIATE2 => 11,
        _ => return None,
    })
}
