use crate::diagnostic::{Diagnostic, Fix, Violation};
use crate::rule_set::Rule;
use air_r_syntax::{RSyntaxKind, RSyntaxToken, RUnaryExpression};
use biome_rowan::{AstNode, TextRange};

pub struct ConsecutiveSigns;

/// <!-- docs: start -->
/// Version added: 0.7.0
///
/// ## What it does
///
/// Reports two or more consecutive `+` or `-` operators in code. Comments and
/// string contents are not checked. Whitespace and comments between operators
/// do not break a sequence, but parentheses do.
///
/// This rule is disabled by default. Enable it with
/// `--select consecutive_signs` or by selecting the `SUSP` category.
///
/// ## Why is this bad?
///
/// R accepts expressions such as `x + + - y` and `--x`, but consecutive signs can
/// be accidental and make the intended operation harder to read.
///
/// Some domain-specific languages intentionally use consecutive signs, such as
/// `igraph::graph_from_literal(A--B)`. Suppress the rule for intentional uses.
///
/// No automatic fix is provided because the intended operation is ambiguous,
/// and arithmetic operators can dispatch to custom methods.
///
/// ## Example
///
/// ```r
/// x + + - y
/// --x
/// ```
///
/// If intentional, make the grouping explicit:
///
/// ```r
/// x + (+y)
/// x - (-y)
/// ```
/// <!-- docs: end -->
impl Violation for ConsecutiveSigns {
    fn rule(&self) -> Rule {
        Rule::ConsecutiveSigns
    }

    fn body(&self) -> String {
        "Consecutive `+` and `-` operators may be a typo.".to_string()
    }
}

fn is_sign(token: &RSyntaxToken) -> bool {
    matches!(token.kind(), RSyntaxKind::PLUS | RSyntaxKind::MINUS)
}

pub fn consecutive_signs(ast: &RUnaryExpression) -> Option<Diagnostic> {
    let operator = ast.operator().ok()?;
    if !is_sign(&operator) {
        return None;
    }

    let previous = operator.prev_token().filter(is_sign);
    // Only the first unary sign reports the chain, including a preceding
    // binary sign. Token adjacency also catches `x + -y * z` across AST levels.
    if previous
        .as_ref()
        .and_then(|token| token.parent())
        .is_some_and(|parent| RUnaryExpression::can_cast(parent.kind()))
    {
        return None;
    }

    let start = previous.as_ref().unwrap_or(&operator);
    let mut end = operator.clone();
    while let Some(next) = end.next_token().filter(is_sign) {
        end = next;
    }
    if start == &end {
        return None;
    }

    let range = TextRange::new(
        start.text_trimmed_range().start(),
        end.text_trimmed_range().end(),
    );
    Some(Diagnostic::new(ConsecutiveSigns, range, Fix::empty()))
}
