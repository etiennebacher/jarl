use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RIfStatement;
use biome_rowan::AstNode;

use crate::lints::coalesce::coalesce::coalesce;
use crate::lints::unnecessary_nesting::unnecessary_nesting::unnecessary_nesting;

pub fn if_(r_expr: &RIfStatement, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    // Check suppressions once for this node
    let suppressed_rules = checker.get_suppressed_rules(node);

    if checker.is_rule_enabled(Rule::Coalesce) && !suppressed_rules.contains(&Rule::Coalesce) {
        checker.report_diagnostic(coalesce(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::UnnecessaryNesting) && !suppressed_rules.contains(&Rule::UnnecessaryNesting) {
        checker.report_diagnostic(unnecessary_nesting(r_expr)?);
    }
    Ok(())
}
