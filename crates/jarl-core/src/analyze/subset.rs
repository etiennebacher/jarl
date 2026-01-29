use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RSubset;
use biome_rowan::AstNode;

use crate::lints::base::sort::sort::sort;

pub fn subset(r_expr: &RSubset, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    // Check suppressions once for this node
    let suppressed_rules = checker.get_suppressed_rules(node);

    if checker.is_rule_enabled(Rule::Sort) && !suppressed_rules.contains(&Rule::Sort) {
        checker.report_diagnostic(sort(r_expr)?);
    }
    Ok(())
}
