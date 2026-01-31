use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RIfStatement;

use crate::lints::base::coalesce::coalesce::coalesce;
use crate::lints::base::if_constant_condition::if_constant_condition::if_constant_condition;
use crate::lints::base::unnecessary_nesting::unnecessary_nesting::unnecessary_nesting;

pub fn if_(r_expr: &RIfStatement, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::Coalesce) {
        checker.report_diagnostic(coalesce(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::IfConstantCondition) {
        checker.report_diagnostic(if_constant_condition(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::UnnecessaryNesting) {
        checker.report_diagnostic(unnecessary_nesting(r_expr)?);
    }
    Ok(())
}
