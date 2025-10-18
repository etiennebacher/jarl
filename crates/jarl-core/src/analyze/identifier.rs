use crate::check::Checker;
use air_r_syntax::RIdentifier;
use biome_rowan::AstNode;

use crate::lints::true_false_symbol::true_false_symbol::true_false_symbol;

pub fn identifier(r_expr: &RIdentifier, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    if checker.is_rule_enabled("true_false_symbol")
        && !checker.should_skip_rule(node, "true_false_symbol")
    {
        checker.report_diagnostic(true_false_symbol(r_expr)?);
    }
    Ok(())
}
