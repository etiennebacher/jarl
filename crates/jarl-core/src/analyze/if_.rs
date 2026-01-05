use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RIfStatement;
use biome_rowan::AstNode;

use crate::lints::coalesce::coalesce::coalesce;

pub fn if_(r_expr: &RIfStatement, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();
    if checker.is_rule_enabled(Rule::Coalesce) && !checker.should_skip_rule(node, Rule::Coalesce) {
        checker.report_diagnostic(coalesce(r_expr)?);
    }
    Ok(())
}
