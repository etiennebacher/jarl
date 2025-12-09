use crate::check::Checker;
use air_r_syntax::RBinaryExpression;
use biome_rowan::AstNode;

use crate::lints::assignment::assignment::assignment;
use crate::lints::class_equals::class_equals::class_equals;
use crate::lints::empty_assignment::empty_assignment::empty_assignment;
use crate::lints::equals_na::equals_na::equals_na;
use crate::lints::implicit_assignment::implicit_assignment::implicit_assignment;
use crate::lints::is_numeric::is_numeric::is_numeric;
use crate::lints::redundant_equals::redundant_equals::redundant_equals;
use crate::lints::seq::seq::seq;
use crate::lints::string_boundary::string_boundary::string_boundary;
use crate::lints::vector_logic::vector_logic::vector_logic;

pub fn binary_expression(r_expr: &RBinaryExpression, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    if checker.is_rule_enabled("assignment") && !checker.should_skip_rule(node, "assignment") {
        checker.report_diagnostic(assignment(r_expr, checker.assignment)?);
    }
    if checker.is_rule_enabled("class_equals") && !checker.should_skip_rule(node, "class_equals") {
        checker.report_diagnostic(class_equals(r_expr)?);
    }
    if checker.is_rule_enabled("vector_logic") && !checker.should_skip_rule(node, "vector_logic") {
        checker.report_diagnostic(vector_logic(r_expr)?);
    }
    if checker.is_rule_enabled("empty_assignment")
        && !checker.should_skip_rule(node, "empty_assignment")
    {
        checker.report_diagnostic(empty_assignment(r_expr)?);
    }
    if checker.is_rule_enabled("equals_na") && !checker.should_skip_rule(node, "equals_na") {
        checker.report_diagnostic(equals_na(r_expr)?);
    }
    if checker.is_rule_enabled("implicit_assignment")
        && !checker.should_skip_rule(node, "implicit_assignment")
    {
        checker.report_diagnostic(implicit_assignment(r_expr)?);
    }
    if checker.is_rule_enabled("is_numeric") && !checker.should_skip_rule(node, "is_numeric") {
        checker.report_diagnostic(is_numeric(r_expr)?);
    }
    if checker.is_rule_enabled("redundant_equals")
        && !checker.should_skip_rule(node, "redundant_equals")
    {
        checker.report_diagnostic(redundant_equals(r_expr)?);
    }
    if checker.is_rule_enabled("seq") && !checker.should_skip_rule(node, "seq") {
        checker.report_diagnostic(seq(r_expr)?);
    }
    if checker.is_rule_enabled("string_boundary")
        && !checker.should_skip_rule(node, "string_boundary")
    {
        checker.report_diagnostic(string_boundary(r_expr)?);
    }
    Ok(())
}
