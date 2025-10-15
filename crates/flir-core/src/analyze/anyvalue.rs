use crate::check::Checker;
use air_r_syntax::AnyRValue;

use crate::lints::numeric_leading_zero::numeric_leading_zero::numeric_leading_zero;

pub fn anyvalue(r_expr: &AnyRValue, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled("numeric_leading_zero") {
        checker.report_diagnostic(numeric_leading_zero(r_expr)?);
    }
    Ok(())
}
