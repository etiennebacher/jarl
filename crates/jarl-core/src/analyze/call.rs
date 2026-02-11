use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RCall;

use crate::lints::base::all_equal::all_equal::all_equal;
use crate::lints::base::any_duplicated::any_duplicated::any_duplicated;
use crate::lints::base::any_is_na::any_is_na::any_is_na;
use crate::lints::base::browser::browser::browser;
use crate::lints::base::class_equals::class_equals::class_identical;
use crate::lints::base::download_file::download_file::download_file;
use crate::lints::base::duplicated_arguments::duplicated_arguments::duplicated_arguments;
use crate::lints::base::fixed_regex::fixed_regex::fixed_regex;
use crate::lints::base::grepv::grepv::grepv;
use crate::lints::base::length_levels::length_levels::length_levels;
use crate::lints::base::length_test::length_test::length_test;
use crate::lints::base::lengths::lengths::lengths;
use crate::lints::base::list2df::list2df::list2df;
use crate::lints::base::matrix_apply::matrix_apply::matrix_apply;
use crate::lints::base::outer_negation::outer_negation::outer_negation;
use crate::lints::base::redundant_ifelse::redundant_ifelse::redundant_ifelse;
use crate::lints::base::sample_int::sample_int::sample_int;
use crate::lints::base::seq2::seq2::seq2;
use crate::lints::base::sprintf::sprintf::sprintf;
use crate::lints::base::system_file::system_file::system_file;
use crate::lints::base::which_grepl::which_grepl::which_grepl;

use crate::lints::testthat::expect_length::expect_length::expect_length;
use crate::lints::testthat::expect_named::expect_named::expect_named;
use crate::lints::testthat::expect_not::expect_not::expect_not;
use crate::lints::testthat::expect_null::expect_null::expect_null;
use crate::lints::testthat::expect_s3_class::expect_s3_class::expect_s3_class;
use crate::lints::testthat::expect_true_false::expect_true_false::expect_true_false;
use crate::lints::testthat::expect_type::expect_type::expect_type;

pub fn call(r_expr: &RCall, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::AllEqual) {
        checker.report_diagnostic(all_equal(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::AnyDuplicated) {
        checker.report_diagnostic(any_duplicated(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::AnyIsNa) {
        checker.report_diagnostic(any_is_na(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::Browser) {
        checker.report_diagnostic(browser(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ClassEquals) {
        checker.report_diagnostic(class_identical(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::DownloadFile) {
        checker.report_diagnostic(download_file(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::DuplicatedArguments) {
        checker.report_diagnostic(duplicated_arguments(r_expr, checker)?);
    }
    if checker.is_rule_enabled(Rule::ExpectLength) {
        checker.report_diagnostic(expect_length(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ExpectNamed) {
        checker.report_diagnostic(expect_named(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ExpectNot) {
        checker.report_diagnostic(expect_not(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ExpectNull) {
        checker.report_diagnostic(expect_null(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ExpectS3Class) {
        checker.report_diagnostic(expect_s3_class(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ExpectType) {
        checker.report_diagnostic(expect_type(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ExpectTrueFalse) {
        checker.report_diagnostic(expect_true_false(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::FixedRegex) {
        checker.report_diagnostic(fixed_regex(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::Grepv) {
        checker.report_diagnostic(grepv(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::LengthLevels) {
        checker.report_diagnostic(length_levels(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::LengthTest) {
        checker.report_diagnostic(length_test(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::Lengths) {
        checker.report_diagnostic(lengths(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::List2df) {
        checker.report_diagnostic(list2df(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::MatrixApply) {
        checker.report_diagnostic(matrix_apply(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::OuterNegation) {
        checker.report_diagnostic(outer_negation(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::RedundantIfelse) {
        checker.report_diagnostic(redundant_ifelse(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::SampleInt) {
        checker.report_diagnostic(sample_int(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::Seq2) {
        checker.report_diagnostic(seq2(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::Sprintf) {
        checker.report_diagnostic(sprintf(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::SystemFile) {
        checker.report_diagnostic(system_file(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::WhichGrepl) {
        checker.report_diagnostic(which_grepl(r_expr)?);
    }
    Ok(())
}
