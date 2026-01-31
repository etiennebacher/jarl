use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RUnaryExpression;

use crate::lints::base::comparison_negation::comparison_negation::comparison_negation;

pub fn unary_expression(r_expr: &RUnaryExpression, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::ComparisonNegation) {
        checker.report_diagnostic(comparison_negation(r_expr)?);
    }
    Ok(())
}
