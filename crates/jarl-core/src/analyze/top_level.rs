use crate::check::Checker;
use crate::lints::unreachable_code::unreachable_code::unreachable_code_top_level;
use crate::rule_set::Rule;
use air_r_syntax::RSyntaxNode;
use biome_rowan::AstNode;

/// Analyze top-level code (all expressions in a file)
///
/// This function runs checks that require analyzing the entire document
/// rather than individual expressions. Currently includes:
/// - Unreachable code detection at the top level
pub fn top_level(
    expressions: &[impl AstNode<Language = air_r_syntax::RLanguage>],
    checker: &mut Checker,
) -> anyhow::Result<()> {
    // Check for unreachable code at the top level
    if checker.is_rule_enabled(Rule::UnreachableCode) {
        let top_level_nodes: Vec<RSyntaxNode> = expressions
            .iter()
            .map(|expr| expr.syntax().clone())
            .collect();

        let diagnostics = unreachable_code_top_level(&top_level_nodes)?;
        for diagnostic in diagnostics {
            checker.report_diagnostic(Some(diagnostic));
        }
    }

    Ok(())
}
