use crate::check::Checker;
use air_r_syntax::RSubset;
use biome_rowan::AstNode;

use crate::lints::sort::sort::sort;

pub fn subset(r_expr: &RSubset, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    if checker.is_rule_enabled("sort") && !checker.should_skip_rule(node, "sort") {
        checker.report_diagnostic(sort(r_expr)?);
    }
    Ok(())
}
