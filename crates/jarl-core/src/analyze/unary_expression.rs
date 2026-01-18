use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RUnaryExpression;
use biome_rowan::AstNode;

use crate::lints::comparison_negation::comparison_negation::comparison_negation;

pub fn unary_expression(r_expr: &RUnaryExpression, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    // Check suppressions once for this node
    let suppressed_rules = checker.get_suppressed_rules(node);

    if checker.is_rule_enabled(Rule::ComparisonNegation)
        && !suppressed_rules.contains(&Rule::ComparisonNegation)
    {
        checker.report_diagnostic(comparison_negation(r_expr)?);
    }
    Ok(())
}
