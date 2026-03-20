use crate::checker::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RIdentifier;

use crate::lints::base::true_false_symbol::true_false_symbol::true_false_symbol;

pub fn identifier(r_expr: &RIdentifier, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::TrueFalseSymbol) {
        checker.report_diagnostic(true_false_symbol(r_expr)?);
    }
    Ok(())
}
