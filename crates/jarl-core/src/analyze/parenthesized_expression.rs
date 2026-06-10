use crate::checker::Checker;
use crate::lints::base::unnecessary_parentheses::unnecessary_parentheses::unnecessary_parentheses;
use crate::rule_set::Rule;
use air_r_syntax::RParenthesizedExpression;

pub fn parenthesized_expression(
    r_expr: &RParenthesizedExpression,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::UnnecessaryParentheses) {
        checker.report_diagnostic(unnecessary_parentheses(r_expr)?);
    }

    Ok(())
}
