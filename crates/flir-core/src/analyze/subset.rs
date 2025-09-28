use crate::check::Checker;
use air_r_syntax::RSubset;

use crate::lints::sort::sort::sort;

pub fn subset(r_expr: &RSubset, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled("sort") {
        checker.report_diagnostic(sort(r_expr)?);
    }
    Ok(())
}
