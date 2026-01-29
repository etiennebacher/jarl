use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RNamespaceExpression;
use biome_rowan::AstNode;

use crate::lints::base::internal_function::internal_function::internal_function;

pub fn namespace_expression(
    r_expr: &RNamespaceExpression,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    // Check suppressions once for this node
    let suppressed_rules = checker.get_suppressed_rules(node);

    if checker.is_rule_enabled(Rule::InternalFunction)
        && !suppressed_rules.contains(&Rule::InternalFunction)
    {
        checker.report_diagnostic(internal_function(r_expr)?);
    }
    Ok(())
}
