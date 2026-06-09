use crate::checker::Checker;
use crate::lints::base::unnecessary_parenthesis::unnecessary_parenthesis::unnecessary_parenthesis;
use crate::rule_set::Rule;
use air_r_syntax::RParenthesizedExpression;

pub fn parenthesized_expression(
    r_expr: &RParenthesizedExpression,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::UnnecessaryParenthesis) {
        checker.report_diagnostic(unnecessary_parenthesis(r_expr)?);
    }

    Ok(())
}
