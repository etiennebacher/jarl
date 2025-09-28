use crate::check::Checker;
use air_r_syntax::RBinaryExpression;

use crate::lints::assignment::assignment::assignment;
use crate::lints::class_equals::class_equals::class_equals;
use crate::lints::empty_assignment::empty_assignment::empty_assignment;
use crate::lints::equals_na::equals_na::equals_na;
use crate::lints::is_numeric::is_numeric::is_numeric;
use crate::lints::redundant_equals::redundant_equals::redundant_equals;

pub fn binary_expression(r_expr: &RBinaryExpression, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled("class_equals") {
        checker.report_diagnostic(class_equals(r_expr)?);
    }
    if checker.is_rule_enabled("empty_assignment") {
        checker.report_diagnostic(empty_assignment(r_expr)?);
    }
    if checker.is_rule_enabled("assignment") {
        checker.report_diagnostic(assignment(r_expr)?);
    }
    if checker.is_rule_enabled("equals_na") {
        checker.report_diagnostic(equals_na(r_expr)?);
    }
    if checker.is_rule_enabled("is_numeric") {
        checker.report_diagnostic(is_numeric(r_expr)?);
    }
    if checker.is_rule_enabled("redundant_equals") {
        checker.report_diagnostic(redundant_equals(r_expr)?);
    }
    Ok(())
}
