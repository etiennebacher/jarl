use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RBinaryExpression;

use crate::lints::base::any_is_na::any_is_na::any_is_na_2;
use crate::lints::base::assignment::assignment::assignment;
use crate::lints::base::class_equals::class_equals::class_equals;
use crate::lints::base::empty_assignment::empty_assignment::empty_assignment;
use crate::lints::base::equals_na::equals_na::equals_na;
use crate::lints::base::equals_nan::equals_nan::equals_nan;
use crate::lints::base::equals_null::equals_null::equals_null;
use crate::lints::base::implicit_assignment::implicit_assignment::implicit_assignment;
use crate::lints::base::is_numeric::is_numeric::is_numeric;
use crate::lints::base::redundant_equals::redundant_equals::redundant_equals;
use crate::lints::base::seq::seq::seq;
use crate::lints::base::string_boundary::string_boundary::string_boundary;
use crate::lints::base::vector_logic::vector_logic::vector_logic;

pub fn binary_expression(r_expr: &RBinaryExpression, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::AnyIsNa) {
        checker.report_diagnostic(any_is_na_2(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::Assignment) {
        checker.report_diagnostic(assignment(
            r_expr,
            checker.rule_options.assignment.operator,
        )?);
    }
    if checker.is_rule_enabled(Rule::ClassEquals) {
        checker.report_diagnostic(class_equals(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::VectorLogic) {
        checker.report_diagnostic(vector_logic(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::EmptyAssignment) {
        checker.report_diagnostic(empty_assignment(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::EqualsNa) {
        checker.report_diagnostic(equals_na(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::EqualsNaN) {
        checker.report_diagnostic(equals_nan(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::EqualsNull) {
        checker.report_diagnostic(equals_null(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ImplicitAssignment) {
        checker.report_diagnostic(implicit_assignment(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::IsNumeric) {
        checker.report_diagnostic(is_numeric(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::RedundantEquals) {
        checker.report_diagnostic(redundant_equals(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::Seq) {
        checker.report_diagnostic(seq(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::StringBoundary) {
        checker.report_diagnostic(string_boundary(r_expr)?);
    }
    Ok(())
}
