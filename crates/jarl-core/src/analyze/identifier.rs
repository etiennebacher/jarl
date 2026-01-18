use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RIdentifier;
use biome_rowan::AstNode;

use crate::lints::true_false_symbol::true_false_symbol::true_false_symbol;

pub fn identifier(r_expr: &RIdentifier, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    // Check suppressions once for this node
    let suppressed_rules = checker.get_suppressed_rules(node);

    if checker.is_rule_enabled(Rule::TrueFalseSymbol)
        && !suppressed_rules.contains(&Rule::TrueFalseSymbol)
    {
        checker.report_diagnostic(true_false_symbol(r_expr)?);
    }
    Ok(())
}
