use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RForStatement;

use crate::lints::base::for_loop_index::for_loop_index::for_loop_index;

pub fn for_loop(r_expr: &RForStatement, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::ForLoopIndex) {
        checker.report_diagnostic(for_loop_index(r_expr)?);
    }
    Ok(())
}
