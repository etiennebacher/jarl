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
/// Reports two or more consecutive `+` or `-` operators in code.
///
/// ## Why is this bad?
///
/// R accepts expressions such as `x + + - y` and `--x`, but consecutive signs can
/// be accidental and make the intended operation harder to read.
///
/// No automatic fix is provided because the intended operation is ambiguous,
/// and arithmetic operators can dispatch to custom methods.
///
/// ## Example
///
/// ```r
/// x + - y
/// --x
/// ```
///
/// Either 1) remove the unnecessary sign, 2) make the grouping more explicit
/// using parenthesis, or 3) add a comment explaining why this is needed.
///
/// ```r
/// x - y
/// -(-x)
/// ```
/// <!-- docs: end -->
impl Violation for ConsecutiveSigns {
    fn rule(&self) -> Rule {
        Rule::ConsecutiveSigns
    }

    fn body(&self) -> String {
        "Consecutive `+` and `-` operators are usually unnecessary and look like a typo."
            .to_string()
    }

    fn suggestion(&self) -> Option<String> {
        Some(
            "Consider removing the unnecessary operators or using parentheses to show that this is intentional."
                .to_string(),
        )
    }
}

fn is_sign(token: &RSyntaxToken) -> bool {
    matches!(token.kind(), RSyntaxKind::PLUS | RSyntaxKind::MINUS)
}

pub fn consecutive_signs(ast: &RUnaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let Ok(operator) = ast.operator() else {
        return Ok(None);
    };
    if !is_sign(&operator) {
        return Ok(None);
    }

    let previous = operator.prev_token().filter(is_sign);
    // Only the first unary sign reports the chain, including a preceding
    // binary sign.
    if previous
        .as_ref()
        .and_then(|token| token.parent())
        .is_some_and(|parent| RUnaryExpression::can_cast(parent.kind()))
    {
        return Ok(None);
    }

    let start = previous.as_ref().unwrap_or(&operator);
    let mut end = operator.clone();
    while let Some(next) = end.next_token().filter(is_sign) {
        end = next;
    }
    if start == &end {
        return Ok(None);
    }

    let range = TextRange::new(
        start.text_trimmed_range().start(),
        end.text_trimmed_range().end(),
    );
    Ok(Some(Diagnostic::new(ConsecutiveSigns, range, Fix::empty())))
}
