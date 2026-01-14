use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RIfStatement;
use biome_rowan::AstNode;

use crate::lints::coalesce::coalesce::coalesce;
use crate::lints::unnecessary_nesting::unnecessary_nesting::unnecessary_nesting;

pub fn if_(r_expr: &RIfStatement, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();
    if checker.is_rule_enabled(Rule::Coalesce) && !checker.should_skip_rule(node, Rule::Coalesce) {
        checker.report_diagnostic(coalesce(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::UnnecessaryNesting)
        && !checker.should_skip_rule(node, Rule::UnnecessaryNesting)
    {
        checker.report_diagnostic(unnecessary_nesting(r_expr)?);
    }
    Ok(())
}
