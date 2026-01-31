use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RSubset;

use crate::lints::base::sort::sort::sort;

/// Run all subset-related lints.
/// Suppressions are handled in post-processing via filter_diagnostics.
pub fn subset(r_expr: &RSubset, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::Sort) {
        checker.report_diagnostic(sort(r_expr)?);
    }
    Ok(())
}
