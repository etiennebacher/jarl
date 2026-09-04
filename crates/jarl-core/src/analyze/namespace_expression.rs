use crate::checker::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RNamespaceExpression;

use crate::lints::base::internal_function::internal_function::internal_function;
use crate::lints::base::undesirable_operator::undesirable_operator::undesirable_operator_namespace;

pub fn namespace_expression(
    r_expr: &RNamespaceExpression,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::InternalFunction) {
        checker.report_diagnostic(internal_function(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::UndesirableOperator) {
        checker.report_diagnostic(undesirable_operator_namespace(
            r_expr,
            &checker.rule_options.undesirable_operator,
        )?);
    }
    Ok(())
}
