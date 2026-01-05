use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RCall;
use biome_rowan::AstNode;

use crate::lints::all_equal::all_equal::all_equal;
use crate::lints::any_duplicated::any_duplicated::any_duplicated;
use crate::lints::any_is_na::any_is_na::any_is_na;
use crate::lints::browser::browser::browser;
use crate::lints::class_equals::class_equals::class_identical;
use crate::lints::download_file::download_file::download_file;
use crate::lints::duplicated_arguments::duplicated_arguments::duplicated_arguments;
use crate::lints::expect_length::expect_length::expect_length;
use crate::lints::expect_named::expect_named::expect_named;
use crate::lints::expect_not::expect_not::expect_not;
use crate::lints::expect_null::expect_null::expect_null;
use crate::lints::expect_s3_class::expect_s3_class::expect_s3_class;
use crate::lints::expect_true_false::expect_true_false::expect_true_false;
use crate::lints::expect_type::expect_type::expect_type;
use crate::lints::fixed_regex::fixed_regex::fixed_regex;
use crate::lints::grepv::grepv::grepv;
use crate::lints::length_levels::length_levels::length_levels;
use crate::lints::length_test::length_test::length_test;
use crate::lints::lengths::lengths::lengths;
use crate::lints::list2df::list2df::list2df;
use crate::lints::matrix_apply::matrix_apply::matrix_apply;
use crate::lints::outer_negation::outer_negation::outer_negation;
use crate::lints::sample_int::sample_int::sample_int;
use crate::lints::seq2::seq2::seq2;
use crate::lints::sprintf::sprintf::sprintf;
use crate::lints::system_file::system_file::system_file;
use crate::lints::which_grepl::which_grepl::which_grepl;

pub fn call(r_expr: &RCall, checker: &mut Checker) -> anyhow::Result<()> {
    let node = r_expr.syntax();

    if checker.is_rule_enabled(Rule::AllEqual) && !checker.should_skip_rule(node, Rule::AllEqual) {
        checker.report_diagnostic(all_equal(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::AnyDuplicated)
        && !checker.should_skip_rule(node, Rule::AnyDuplicated)
    {
        checker.report_diagnostic(any_duplicated(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::AnyIsNa) && !checker.should_skip_rule(node, Rule::AnyIsNa) {
        checker.report_diagnostic(any_is_na(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::Browser) && !checker.should_skip_rule(node, Rule::Browser) {
        checker.report_diagnostic(browser(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ClassEquals)
        && !checker.should_skip_rule(node, Rule::ClassEquals)
    {
        checker.report_diagnostic(class_identical(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::DownloadFile)
        && !checker.should_skip_rule(node, Rule::DownloadFile)
    {
        checker.report_diagnostic(download_file(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::DuplicatedArguments)
        && !checker.should_skip_rule(node, Rule::DuplicatedArguments)
    {
        checker.report_diagnostic(duplicated_arguments(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ExpectLength)
        && !checker.should_skip_rule(node, Rule::ExpectLength)
    {
        checker.report_diagnostic(expect_length(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ExpectNamed)
        && !checker.should_skip_rule(node, Rule::ExpectNamed)
    {
        checker.report_diagnostic(expect_named(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ExpectNot) && !checker.should_skip_rule(node, Rule::ExpectNot)
    {
        checker.report_diagnostic(expect_not(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ExpectNull)
        && !checker.should_skip_rule(node, Rule::ExpectNull)
    {
        checker.report_diagnostic(expect_null(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ExpectS3Class)
        && !checker.should_skip_rule(node, Rule::ExpectS3Class)
    {
        checker.report_diagnostic(expect_s3_class(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ExpectType)
        && !checker.should_skip_rule(node, Rule::ExpectType)
    {
        checker.report_diagnostic(expect_type(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::ExpectTrueFalse)
        && !checker.should_skip_rule(node, Rule::ExpectTrueFalse)
    {
        checker.report_diagnostic(expect_true_false(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::FixedRegex)
        && !checker.should_skip_rule(node, Rule::FixedRegex)
    {
        checker.report_diagnostic(fixed_regex(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::Grepv) && !checker.should_skip_rule(node, Rule::Grepv) {
        checker.report_diagnostic(grepv(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::LengthLevels)
        && !checker.should_skip_rule(node, Rule::LengthLevels)
    {
        checker.report_diagnostic(length_levels(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::LengthTest)
        && !checker.should_skip_rule(node, Rule::LengthTest)
    {
        checker.report_diagnostic(length_test(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::Lengths) && !checker.should_skip_rule(node, Rule::Lengths) {
        checker.report_diagnostic(lengths(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::List2df) && !checker.should_skip_rule(node, Rule::List2df) {
        checker.report_diagnostic(list2df(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::MatrixApply)
        && !checker.should_skip_rule(node, Rule::MatrixApply)
    {
        checker.report_diagnostic(matrix_apply(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::OuterNegation)
        && !checker.should_skip_rule(node, Rule::OuterNegation)
    {
        checker.report_diagnostic(outer_negation(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::SampleInt) && !checker.should_skip_rule(node, Rule::SampleInt)
    {
        checker.report_diagnostic(sample_int(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::Seq2) && !checker.should_skip_rule(node, Rule::Seq2) {
        checker.report_diagnostic(seq2(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::Sprintf) && !checker.should_skip_rule(node, Rule::Sprintf) {
        checker.report_diagnostic(sprintf(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::SystemFile)
        && !checker.should_skip_rule(node, Rule::SystemFile)
    {
        checker.report_diagnostic(system_file(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::WhichGrepl)
        && !checker.should_skip_rule(node, Rule::WhichGrepl)
    {
        checker.report_diagnostic(which_grepl(r_expr)?);
    }
    Ok(())
}
