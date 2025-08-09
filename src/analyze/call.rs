use crate::check_expression::Checker;
use air_r_syntax::RCall;

use crate::lints::any_duplicated::any_duplicated::any_duplicated;
use crate::lints::any_is_na::any_is_na::any_is_na;
use crate::lints::duplicated_arguments::duplicated_arguments::duplicated_arguments;
use crate::lints::length_levels::length_levels::length_levels;
use crate::lints::length_test::length_test::length_test;
use crate::lints::lengths::lengths::lengths;
use crate::lints::which_grepl::which_grepl::which_grepl;

pub fn call(r_expr: &RCall, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled("any_duplicated") {
        checker.report_diagnostic(any_duplicated(r_expr)?);
    }
    if checker.is_rule_enabled("any_is_na") {
        checker.report_diagnostic(any_is_na(r_expr)?);
    }
    if checker.is_rule_enabled("duplicated_arguments") {
        checker.report_diagnostic(duplicated_arguments(r_expr)?);
    }
    if checker.is_rule_enabled("length_levels") {
        checker.report_diagnostic(length_levels(r_expr)?);
    }
    if checker.is_rule_enabled("length_test") {
        checker.report_diagnostic(length_test(r_expr)?);
    }
    if checker.is_rule_enabled("lengths") {
        checker.report_diagnostic(lengths(r_expr)?);
    }
    if checker.is_rule_enabled("which_grepl") {
        checker.report_diagnostic(which_grepl(r_expr)?);
    }
    Ok(())
}
