use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RForStatement;

use crate::lints::base::for_loop_index::for_loop_index::for_loop_index;

/// Run all for loop-related lints.
/// Suppressions are handled in post-processing via filter_diagnostics.
pub fn for_loop(r_expr: &RForStatement, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::ForLoopIndex) {
        checker.report_diagnostic(for_loop_index(r_expr)?);
    }
    Ok(())
}
