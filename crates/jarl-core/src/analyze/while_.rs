use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RWhileStatement;
use biome_rowan::AstNode;

use crate::lints::repeat::repeat::repeat;

pub fn while_(r_expr: &RWhileStatement, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    // Check suppressions once for this node
    let suppressed_rules = checker.get_suppressed_rules(node);

    if checker.is_rule_enabled(Rule::Repeat) && !suppressed_rules.contains(&Rule::Repeat) {
        checker.report_diagnostic(repeat(r_expr)?);
    }
    Ok(())
}
