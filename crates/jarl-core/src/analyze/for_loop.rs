use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RForStatement;
use biome_rowan::AstNode;

use crate::lints::for_loop_index::for_loop_index::for_loop_index;

pub fn for_loop(r_expr: &RForStatement, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    // Check suppressions once for this node
    let suppressed_rules = checker.get_suppressed_rules(node);

    if checker.is_rule_enabled(Rule::ForLoopIndex)
        && !suppressed_rules.contains(&Rule::ForLoopIndex)
    {
        checker.report_diagnostic(for_loop_index(r_expr)?);
    }
    Ok(())
}
