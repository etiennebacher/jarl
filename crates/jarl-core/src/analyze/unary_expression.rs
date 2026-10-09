use crate::checker::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RUnaryExpression;

use crate::lints::base::comparison_negation::comparison_negation::comparison_negation;
use crate::lints::base::consecutive_signs::consecutive_signs::consecutive_signs;
use crate::lints::base::notin::notin::notin;

pub fn unary_expression(r_expr: &RUnaryExpression, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::ComparisonNegation) {
        checker.report_diagnostic(comparison_negation(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ConsecutiveSigns) {
        checker.report_diagnostic(consecutive_signs(r_expr));
    }
    if checker.is_rule_enabled(Rule::NotIn) {
        checker.report_diagnostic(notin(r_expr)?);
    }
    Ok(())
}
