use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RWhileStatement;

use crate::lints::repeat::repeat::repeat;

pub fn while_(r_expr: &RWhileStatement, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::Repeat) {
        checker.report_diagnostic(repeat(r_expr)?);
    }
    Ok(())
}
