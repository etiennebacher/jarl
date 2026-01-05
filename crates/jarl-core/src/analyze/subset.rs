use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RSubset;
use biome_rowan::AstNode;

use crate::lints::sort::sort::sort;

pub fn subset(r_expr: &RSubset, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    if checker.is_rule_enabled(Rule::Sort) && !checker.should_skip_rule(node, Rule::Sort) {
        checker.report_diagnostic(sort(r_expr)?);
    }
    Ok(())
}
