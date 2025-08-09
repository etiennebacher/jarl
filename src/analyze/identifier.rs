use crate::check_ast::Checker;
use crate::trait_lint_checker::LintChecker;
use air_r_syntax::{AnyRExpression, RIdentifier};

use crate::lints::true_false_symbol::true_false_symbol::TrueFalseSymbol;

pub fn identifier(r_expr: &RIdentifier, checker: &mut Checker) -> anyhow::Result<()> {
    let any_r_exp: &AnyRExpression = &r_expr.clone().into();
    if checker.is_enabled("true_false_symbol") {
        checker.report_diagnostic(TrueFalseSymbol.check(any_r_exp)?);
    }
    Ok(())
}
