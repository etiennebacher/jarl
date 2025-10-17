use crate::check::Checker;
use air_r_syntax::RIfStatement;

use crate::lints::coalesce::coalesce::coalesce;

pub fn if_(r_expr: &RIfStatement, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled("coalesce") {
        checker.report_diagnostic(coalesce(r_expr)?);
    }
    Ok(())
}
