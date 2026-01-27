use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RBinaryExpression;
use biome_rowan::AstNode;

use crate::lints::any_is_na::any_is_na::any_is_na_2;
use crate::lints::assignment::assignment::assignment;
use crate::lints::class_equals::class_equals::class_equals;
use crate::lints::empty_assignment::empty_assignment::empty_assignment;
use crate::lints::equals_na::equals_na::equals_na;
use crate::lints::equals_nan::equals_nan::equals_nan;
use crate::lints::equals_null::equals_null::equals_null;
use crate::lints::implicit_assignment::implicit_assignment::implicit_assignment;
use crate::lints::is_numeric::is_numeric::is_numeric;
use crate::lints::redundant_equals::redundant_equals::redundant_equals;
use crate::lints::seq::seq::seq;
use crate::lints::string_boundary::string_boundary::string_boundary;
use crate::lints::vector_logic::vector_logic::vector_logic;

pub fn binary_expression(r_expr: &RBinaryExpression, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    // Check suppressions once for this node
    let suppressed_rules = checker.get_suppressed_rules(node);

    if checker.is_rule_enabled(Rule::AnyIsNa) && !suppressed_rules.contains(&Rule::AnyIsNa) {
        checker.report_diagnostic(any_is_na_2(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::Assignment) && !suppressed_rules.contains(&Rule::Assignment) {
        checker.report_diagnostic(assignment(r_expr, checker.assignment)?);
    }
    if checker.is_rule_enabled(Rule::ClassEquals) && !suppressed_rules.contains(&Rule::ClassEquals)
    {
        checker.report_diagnostic(class_equals(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::VectorLogic) && !suppressed_rules.contains(&Rule::VectorLogic)
    {
        checker.report_diagnostic(vector_logic(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::EmptyAssignment)
        && !suppressed_rules.contains(&Rule::EmptyAssignment)
    {
        checker.report_diagnostic(empty_assignment(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::EqualsNa) && !suppressed_rules.contains(&Rule::EqualsNa) {
        checker.report_diagnostic(equals_na(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::EqualsNaN) && !suppressed_rules.contains(&Rule::EqualsNaN) {
        checker.report_diagnostic(equals_nan(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::EqualsNull) && !suppressed_rules.contains(&Rule::EqualsNull) {
        checker.report_diagnostic(equals_null(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ImplicitAssignment)
        && !suppressed_rules.contains(&Rule::ImplicitAssignment)
    {
        checker.report_diagnostic(implicit_assignment(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::IsNumeric) && !suppressed_rules.contains(&Rule::IsNumeric) {
        checker.report_diagnostic(is_numeric(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::RedundantEquals)
        && !suppressed_rules.contains(&Rule::RedundantEquals)
    {
        checker.report_diagnostic(redundant_equals(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::Seq) && !suppressed_rules.contains(&Rule::Seq) {
        checker.report_diagnostic(seq(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::StringBoundary)
        && !suppressed_rules.contains(&Rule::StringBoundary)
    {
        checker.report_diagnostic(string_boundary(r_expr)?);
    }
    Ok(())
}
