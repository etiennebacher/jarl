use crate::diagnostic::*;
use air_r_syntax::*;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for usage of `:::`.
///
/// ## Why is this bad?
///
/// Using `:::` to access a package's internal functions is unsafe. Those
/// functions are not part of the package's public interface and may be changed
/// or removed by the maintainers without notice. Use public functions via `::`
/// instead.
///
/// This rule doesn't have an automatic fix.
pub fn internal_function(ast: &RNamespaceExpression) -> anyhow::Result<Option<Diagnostic>> {
    let op = ast.operator()?;
    if op.kind() != RSyntaxKind::COLON3 {
        return Ok(None);
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "internal_function".to_string(),
            "Accessing a package's internal function with `:::` is likely to break in the future."
                .to_string(),
            Some("Use public functions via `::` instead.".to_string()),
        ),
        range,
        Fix::empty(),
    );

    Ok(Some(diagnostic))
}
