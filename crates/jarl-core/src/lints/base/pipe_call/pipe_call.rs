use crate::diagnostic::*;
use crate::rule_set::Rule;
use air_r_syntax::*;
use biome_rowan::AstNode;

/// <!-- docs: start -->
/// Version added: 0.7.0
///
/// ## What it does
///
/// Reports bare function names on the right-hand side of magrittr pipes.
///
/// ## Why is this bad?
///
/// An explicit function call makes each step of a pipe easier to read.
///
/// This rule is disabled by default.
///
/// ## Example
///
/// ```r
/// x %>% sum
/// ```
///
/// Use instead:
///
/// ```r
/// x %>% sum()
/// ```
/// <!-- docs: end -->
pub fn pipe_call(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left: _, operator, right } = ast.as_fields();
    let operator = operator?;

    // `%$%` exposes names rather than applying a function.
    if operator.kind() != RSyntaxKind::SPECIAL
        || !matches!(operator.text_trimmed(), "%>%" | "%!>%" | "%T>%" | "%<>%")
    {
        return Ok(None);
    }

    // Check if the right-hand side is an identifier (e.g., `x`),
    // rather than a function call (e.g., `x()`).
    let AnyRExpression::RIdentifier(right) = right? else {
        return Ok(None);
    };
    let name = right.to_trimmed_string();
    let range = right.syntax().text_trimmed_range();
    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::PipeCall,
            "Use an explicit call on the right-hand side of a magrittr pipe.".to_string(),
            Some(format!("Use `{name}()` instead.")),
        ),
        range,
        Fix::new(range, format!("{name}()"), false),
    )))
}
