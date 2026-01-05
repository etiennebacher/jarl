use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RIdentifier;
use biome_rowan::AstNode;

use crate::lints::true_false_symbol::true_false_symbol::true_false_symbol;

pub fn identifier(r_expr: &RIdentifier, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    if checker.is_rule_enabled(Rule::TrueFalseSymbol)
        && !checker.should_skip_rule(node, Rule::TrueFalseSymbol)
    {
        checker.report_diagnostic(true_false_symbol(r_expr)?);
    }
    Ok(())
}
