use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RUnaryExpression;
use biome_rowan::AstNode;

use crate::lints::comparison_negation::comparison_negation::comparison_negation;

pub fn unary_expression(r_expr: &RUnaryExpression, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();
    if checker.is_rule_enabled(Rule::ComparisonNegation)
        && !checker.should_skip_rule(node, Rule::ComparisonNegation)
    {
        checker.report_diagnostic(comparison_negation(r_expr)?);
    }
    Ok(())
}
