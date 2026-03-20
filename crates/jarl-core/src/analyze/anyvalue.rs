use crate::checker::Checker;
use crate::rule_set::Rule;
use air_r_syntax::AnyRValue;

use crate::lints::base::numeric_leading_zero::numeric_leading_zero::numeric_leading_zero;
use crate::lints::base::quotes::quotes::quotes;

pub fn anyvalue(r_expr: &AnyRValue, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::NumericLeadingZero) {
        checker.report_diagnostic(numeric_leading_zero(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::Quotes) {
        checker.report_diagnostic(quotes(
            r_expr,
            checker.rule_options.quotes.preferred_delimiter,
        )?);
    }
    Ok(())
}
