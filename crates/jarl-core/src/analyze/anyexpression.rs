use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::AnyRExpression;

use crate::lints::comments::blanket_suppression::blanket_suppression::blanket_suppression;

pub fn anyexpression(r_expr: &AnyRExpression, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::BlanketSuppression) {
        let comments = &checker.suppression.comments;
        checker.report_diagnostic(blanket_suppression(r_expr, comments)?);
    }
    Ok(())
}
