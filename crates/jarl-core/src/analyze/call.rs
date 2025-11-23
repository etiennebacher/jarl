use crate::check::Checker;
use air_r_syntax::RCall;
use biome_rowan::AstNode;

use crate::lints::all_equal::all_equal::all_equal;
use crate::lints::any_duplicated::any_duplicated::any_duplicated;
use crate::lints::any_is_na::any_is_na::any_is_na;
use crate::lints::browser::browser::browser;
use crate::lints::download_file::download_file::download_file;
use crate::lints::duplicated_arguments::duplicated_arguments::duplicated_arguments;
use crate::lints::grepv::grepv::grepv;
use crate::lints::length_levels::length_levels::length_levels;
use crate::lints::length_test::length_test::length_test;
use crate::lints::lengths::lengths::lengths;
use crate::lints::list2df::list2df::list2df;
use crate::lints::matrix_apply::matrix_apply::matrix_apply;
use crate::lints::outer_negation::outer_negation::outer_negation;
use crate::lints::sample_int::sample_int::sample_int;
use crate::lints::system_file::system_file::system_file;
use crate::lints::which_grepl::which_grepl::which_grepl;

pub fn call(r_expr: &RCall, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    if checker.is_rule_enabled("all_equal") && !checker.should_skip_rule(node, "all_equal") {
        checker.report_diagnostic(all_equal(r_expr)?);
    }
    if checker.is_rule_enabled("any_duplicated")
        && !checker.should_skip_rule(node, "any_duplicated")
    {
        checker.report_diagnostic(any_duplicated(r_expr)?);
    }
    if checker.is_rule_enabled("any_is_na") && !checker.should_skip_rule(node, "any_is_na") {
        checker.report_diagnostic(any_is_na(r_expr)?);
    }
    if checker.is_rule_enabled("browser") && !checker.should_skip_rule(node, "browser") {
        checker.report_diagnostic(browser(r_expr)?);
    }
    if checker.is_rule_enabled("download_file") && !checker.should_skip_rule(node, "download_file")
    {
        checker.report_diagnostic(download_file(r_expr)?);
    }
    if checker.is_rule_enabled("duplicated_arguments")
        && !checker.should_skip_rule(node, "duplicated_arguments")
    {
        checker.report_diagnostic(duplicated_arguments(r_expr)?);
    }
    if checker.is_rule_enabled("grepv") && !checker.should_skip_rule(node, "grepv") {
        checker.report_diagnostic(grepv(r_expr)?);
    }
    if checker.is_rule_enabled("length_levels") && !checker.should_skip_rule(node, "length_levels")
    {
        checker.report_diagnostic(length_levels(r_expr)?);
    }
    if checker.is_rule_enabled("length_test") && !checker.should_skip_rule(node, "length_test") {
        checker.report_diagnostic(length_test(r_expr)?);
    }
    if checker.is_rule_enabled("lengths") && !checker.should_skip_rule(node, "lengths") {
        checker.report_diagnostic(lengths(r_expr)?);
    }
    if checker.is_rule_enabled("list2df") && !checker.should_skip_rule(node, "list2df") {
        checker.report_diagnostic(list2df(r_expr)?);
    }
    if checker.is_rule_enabled("matrix_apply") && !checker.should_skip_rule(node, "matrix_apply") {
        checker.report_diagnostic(matrix_apply(r_expr)?);
    }
    if checker.is_rule_enabled("outer_negation")
        && !checker.should_skip_rule(node, "outer_negation")
    {
        checker.report_diagnostic(outer_negation(r_expr)?);
    }
    if checker.is_rule_enabled("sample_int") && !checker.should_skip_rule(node, "sample_int") {
        checker.report_diagnostic(sample_int(r_expr)?);
    }
    if checker.is_rule_enabled("system_file") && !checker.should_skip_rule(node, "system_file") {
        checker.report_diagnostic(system_file(r_expr)?);
    }
    if checker.is_rule_enabled("which_grepl") && !checker.should_skip_rule(node, "which_grepl") {
        checker.report_diagnostic(which_grepl(r_expr)?);
    }
    Ok(())
}
