use crate::checker::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RSubset;

use crate::lints::base::negative_index::negative_index::negative_index;
use crate::lints::base::sort::sort::sort;

pub fn subset(r_expr: &RSubset, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::NegativeIndex) {
        for diagnostic in negative_index(r_expr)? {
            checker.report_diagnostic(Some(diagnostic));
        }
    }
    if checker.is_rule_enabled(Rule::Sort) {
        checker.report_diagnostic(sort(r_expr)?);
    }
    Ok(())
}
