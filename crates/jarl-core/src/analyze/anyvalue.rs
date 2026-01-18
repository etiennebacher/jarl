use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::AnyRValue;
use biome_rowan::AstNode;

use crate::lints::numeric_leading_zero::numeric_leading_zero::numeric_leading_zero;

pub fn anyvalue(r_expr: &AnyRValue, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    // Check suppressions once for this node
    let suppressed_rules = checker.get_suppressed_rules(node);

    if checker.is_rule_enabled(Rule::NumericLeadingZero) && !suppressed_rules.contains(&Rule::NumericLeadingZero) {
        checker.report_diagnostic(numeric_leading_zero(r_expr)?);
    }
    Ok(())
}
