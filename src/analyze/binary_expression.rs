use crate::check_ast::Checker;
use crate::trait_lint_checker::LintChecker;
use air_r_syntax::{AnyRExpression, RBinaryExpression};

use crate::lints::class_equals::class_equals::ClassEquals;
use crate::lints::empty_assignment::empty_assignment::EmptyAssignment;
use crate::lints::equal_assignment::equal_assignment::EqualAssignment;
use crate::lints::equals_na::equals_na::EqualsNa;
use crate::lints::redundant_equals::redundant_equals::RedundantEquals;

pub fn binary_expression(r_expr: &RBinaryExpression, checker: &mut Checker) -> anyhow::Result<()> {
    let any_r_exp: &AnyRExpression = &r_expr.clone().into();
    if checker.is_enabled("class_equals") {
        checker.report_diagnostic(ClassEquals.check(any_r_exp)?);
    }
    if checker.is_enabled("empty_assignment") {
        checker.report_diagnostic(EmptyAssignment.check(any_r_exp)?);
    }
    if checker.is_enabled("equal_assignment") {
        checker.report_diagnostic(EqualAssignment.check(any_r_exp)?);
    }
    if checker.is_enabled("equals_na") {
        checker.report_diagnostic(EqualsNa.check(any_r_exp)?);
    }
    if checker.is_enabled("redundant_equals") {
        checker.report_diagnostic(RedundantEquals.check(any_r_exp)?);
    }
    Ok(())
}
