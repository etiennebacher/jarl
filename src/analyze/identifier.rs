use crate::check_expression::Checker;
use air_r_syntax::RIdentifier;

use crate::lints::true_false_symbol::true_false_symbol::true_false_symbol;

pub fn identifier(r_expr: &RIdentifier, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled("true_false_symbol") {
        checker.report_diagnostic(true_false_symbol(r_expr)?);
    }
    Ok(())
}
