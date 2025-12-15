use crate::check::Checker;
use air_r_syntax::AnyRExpression;

use crate::lints::blanket_suppression::blanket_suppression::blanket_suppression;

pub fn anyexpression(r_expr: &AnyRExpression, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled("blanket_suppression") {
        checker.report_diagnostic(blanket_suppression(r_expr)?);
    }
    Ok(())
}
