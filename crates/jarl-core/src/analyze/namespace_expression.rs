use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RNamespaceExpression;

use crate::lints::base::internal_function::internal_function::internal_function;

/// Run all namespace expression-related lints.
/// Suppressions are handled in post-processing via filter_diagnostics.
pub fn namespace_expression(
    r_expr: &RNamespaceExpression,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::InternalFunction) {
        checker.report_diagnostic(internal_function(r_expr)?);
    }
    Ok(())
}
