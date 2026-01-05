use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::AnyRValue;
use biome_rowan::AstNode;

use crate::lints::numeric_leading_zero::numeric_leading_zero::numeric_leading_zero;

pub fn anyvalue(r_expr: &AnyRValue, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    if checker.is_rule_enabled(Rule::NumericLeadingZero)
        && !checker.should_skip_rule(node, Rule::NumericLeadingZero)
    {
        checker.report_diagnostic(numeric_leading_zero(r_expr)?);
    }
    Ok(())
}
