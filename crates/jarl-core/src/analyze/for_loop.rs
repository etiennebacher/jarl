use crate::check::Checker;
use air_r_syntax::RForStatement;
use biome_rowan::AstNode;

use crate::lints::for_loop_index::for_loop_index::for_loop_index;

pub fn for_loop(r_expr: &RForStatement, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    if checker.is_rule_enabled("for_loop_index")
        && !checker.should_skip_rule(node, "for_loop_index")
    {
        checker.report_diagnostic(for_loop_index(r_expr)?);
    }
    Ok(())
}
