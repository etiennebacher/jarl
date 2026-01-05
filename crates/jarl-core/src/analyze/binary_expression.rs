use crate::check::Checker;
use crate::rule_set::Rule;
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

    if checker.is_rule_enabled(Rule::Assignment)
        && !checker.should_skip_rule(node, Rule::Assignment)
    {
        checker.report_diagnostic(assignment(r_expr, checker.assignment)?);
    }
    if checker.is_rule_enabled(Rule::ClassEquals)
        && !checker.should_skip_rule(node, Rule::ClassEquals)
    {
        checker.report_diagnostic(class_equals(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::VectorLogic)
        && !checker.should_skip_rule(node, Rule::VectorLogic)
    {
        checker.report_diagnostic(vector_logic(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::EmptyAssignment)
        && !checker.should_skip_rule(node, Rule::EmptyAssignment)
    {
        checker.report_diagnostic(empty_assignment(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::EqualsNa) && !checker.should_skip_rule(node, Rule::EqualsNa) {
        checker.report_diagnostic(equals_na(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ImplicitAssignment)
        && !checker.should_skip_rule(node, Rule::ImplicitAssignment)
    {
        checker.report_diagnostic(implicit_assignment(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::IsNumeric) && !checker.should_skip_rule(node, Rule::IsNumeric)
    {
        checker.report_diagnostic(is_numeric(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::RedundantEquals)
        && !checker.should_skip_rule(node, Rule::RedundantEquals)
    {
        checker.report_diagnostic(redundant_equals(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::Seq) && !checker.should_skip_rule(node, Rule::Seq) {
        checker.report_diagnostic(seq(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::StringBoundary)
        && !checker.should_skip_rule(node, Rule::StringBoundary)
    {
        checker.report_diagnostic(string_boundary(r_expr)?);
    }
    Ok(())
}
