use crate::check_ast::Checker;
use crate::trait_lint_checker::LintChecker;
use air_r_syntax::{AnyRExpression, RCall};

use crate::lints::any_duplicated::any_duplicated::AnyDuplicated;
use crate::lints::any_is_na::any_is_na::AnyIsNa;
use crate::lints::duplicated_arguments::duplicated_arguments::DuplicatedArguments;
use crate::lints::length_levels::length_levels::LengthLevels;
use crate::lints::length_test::length_test::LengthTest;
use crate::lints::lengths::lengths::Lengths;
use crate::lints::which_grepl::which_grepl::WhichGrepl;

pub fn call(r_expr: &RCall, checker: &mut Checker) -> anyhow::Result<()> {
    let any_r_exp: &AnyRExpression = &r_expr.clone().into();
    if checker.is_enabled("any_duplicated") {
        checker.report_diagnostic(AnyDuplicated.check(any_r_exp)?);
    }
    if checker.is_enabled("any_is_na") {
        checker.report_diagnostic(AnyIsNa.check(any_r_exp)?);
    }
    if checker.is_enabled("duplicated_arguments") {
        checker.report_diagnostic(DuplicatedArguments.check(any_r_exp)?);
    }
    if checker.is_enabled("length_levels") {
        checker.report_diagnostic(LengthLevels.check(any_r_exp)?);
    }
    if checker.is_enabled("length_test") {
        checker.report_diagnostic(LengthTest.check(any_r_exp)?);
    }
    if checker.is_enabled("lengths") {
        checker.report_diagnostic(Lengths.check(any_r_exp)?);
    }
    if checker.is_enabled("which_grepl") {
        checker.report_diagnostic(WhichGrepl.check(any_r_exp)?);
    }
    Ok(())
}
