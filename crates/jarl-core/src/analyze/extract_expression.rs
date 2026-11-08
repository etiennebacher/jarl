use crate::checker::Checker;
use crate::lints::base::undesirable_operator::undesirable_operator::undesirable_operator_extract;
use crate::rule_set::Rule;
use air_r_syntax::RExtractExpression;

pub fn extract_expression(
    r_expr: &RExtractExpression,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::UndesirableOperator) {
        checker.report_diagnostic(undesirable_operator_extract(
            r_expr,
            &checker.rule_options.undesirable_operator,
        )?);
    }
    Ok(())
}
