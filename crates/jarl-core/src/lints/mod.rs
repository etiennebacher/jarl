use crate::rule_table::{DefaultStatus, FixStatus, RuleTable};
use std::collections::HashSet;
use std::sync::OnceLock;

pub(crate) mod all_equal;
pub(crate) mod any_duplicated;
pub(crate) mod any_is_na;
pub(crate) mod assignment;
pub(crate) mod browser;
pub(crate) mod class_equals;
pub(crate) mod coalesce;
pub(crate) mod comparison_negation;
pub(crate) mod download_file;
pub(crate) mod duplicated_arguments;
pub(crate) mod empty_assignment;
pub(crate) mod equals_na;
pub(crate) mod expect_length;
pub(crate) mod expect_named;
pub(crate) mod expect_not;
pub(crate) mod expect_null;
pub(crate) mod expect_s3_class;
pub(crate) mod expect_true_false;
pub(crate) mod expect_type;
pub(crate) mod fixed_regex;
pub(crate) mod for_loop_index;
pub(crate) mod grepv;
pub(crate) mod implicit_assignment;
pub(crate) mod is_numeric;
pub(crate) mod length_levels;
pub(crate) mod length_test;
pub(crate) mod lengths;
pub(crate) mod list2df;
pub(crate) mod matrix_apply;
pub(crate) mod numeric_leading_zero;
pub(crate) mod outer_negation;
pub(crate) mod redundant_equals;
pub(crate) mod repeat;
pub(crate) mod sample_int;
pub(crate) mod seq;
pub(crate) mod seq2;
pub(crate) mod sort;
pub(crate) mod sprintf;
pub(crate) mod string_boundary;
pub(crate) mod system_file;
pub(crate) mod true_false_symbol;
pub(crate) mod vector_logic;
pub(crate) mod which_grepl;

pub static RULE_GROUPS: &[&str] = &["CORR", "PERF", "READ", "SUSP", "TESTTHAT"];

/// List of supported rules and additional metadata, including whether they have
/// a fix, the categories they belong to, and whether they are enabled by default.
///
/// Possible categories:
/// - CORR: correctness, code that is outright wrong or useless
/// - SUSP: suspicious, code that is most likely wrong or useless
/// - PERF: performance, code that can be written to run faster
/// - READ: readibility, code is correct but can be written in a way that is
///   easier to read.
pub fn all_rules_and_safety() -> RuleTable {
    let mut rule_table = RuleTable::empty();
    rule_table.add_rule(
        "all_equal",
        "SUSP",
        DefaultStatus::Enabled,
        FixStatus::Unsafe,
        None,
    );
    rule_table.add_rule(
        "any_duplicated",
        "PERF",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "any_is_na",
        "PERF",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "assignment",
        "READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "browser",
        "CORR",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "class_equals",
        "SUSP",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "comparison_negation",
        "READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "coalesce",
        "READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        Some((4, 4, 0)),
    );
    rule_table.add_rule(
        "download_file",
        "SUSP",
        DefaultStatus::Enabled,
        FixStatus::None,
        None,
    );
    rule_table.add_rule(
        "duplicated_arguments",
        "SUSP",
        DefaultStatus::Enabled,
        FixStatus::None,
        None,
    );
    rule_table.add_rule(
        "empty_assignment",
        "READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "equals_na",
        "CORR",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "expect_length",
        "TESTTHAT",
        DefaultStatus::Disabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "expect_named",
        "TESTTHAT",
        DefaultStatus::Disabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "expect_not",
        "TESTTHAT",
        DefaultStatus::Disabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "expect_null",
        "TESTTHAT",
        DefaultStatus::Disabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "expect_s3_class",
        "TESTTHAT",
        DefaultStatus::Disabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "expect_true_false",
        "TESTTHAT",
        DefaultStatus::Disabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "expect_type",
        "TESTTHAT",
        DefaultStatus::Disabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "fixed_regex",
        "READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "for_loop_index",
        "READ",
        DefaultStatus::Enabled,
        FixStatus::None,
        None,
    );
    rule_table.add_rule(
        "grepv",
        "READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        Some((4, 5, 0)),
    );
    rule_table.add_rule(
        "implicit_assignment",
        "READ",
        DefaultStatus::Enabled,
        FixStatus::None,
        None,
    );
    rule_table.add_rule(
        "is_numeric",
        "READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "length_levels",
        "READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "length_test",
        "CORR",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "lengths",
        "PERF,READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "list2df",
        "PERF,READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        Some((4, 0, 0)),
    );
    rule_table.add_rule(
        "matrix_apply",
        "PERF",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "numeric_leading_zero",
        "READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "outer_negation",
        "PERF,READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "redundant_equals",
        "READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "repeat",
        "READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "sample_int",
        "READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule("seq", "SUSP", DefaultStatus::Enabled, FixStatus::Safe, None);
    rule_table.add_rule(
        "seq2",
        "SUSP",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "sort",
        "PERF,READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "sprintf",
        "CORR,SUSP",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "string_boundary",
        "PERF, READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "system_file",
        "READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table.add_rule(
        "true_false_symbol",
        "READ",
        DefaultStatus::Enabled,
        FixStatus::None,
        None,
    );
    rule_table.add_rule(
        "vector_logic",
        "PERF",
        DefaultStatus::Enabled,
        FixStatus::None,
        None,
    );
    rule_table.add_rule(
        "which_grepl",
        "PERF,READ",
        DefaultStatus::Enabled,
        FixStatus::Safe,
        None,
    );
    rule_table
}

/// Cached set of safe rule names for O(1) lookup
static SAFE_RULES: OnceLock<HashSet<String>> = OnceLock::new();

/// Cached set of unsafe rule names for O(1) lookup
static UNSAFE_RULES: OnceLock<HashSet<String>> = OnceLock::new();

/// Cached set of no-fix rule names for O(1) lookup
static NOFIX_RULES: OnceLock<HashSet<String>> = OnceLock::new();

/// Get the cached set of safe rule names
pub fn safe_rules_set() -> &'static HashSet<String> {
    SAFE_RULES.get_or_init(|| {
        all_rules_and_safety()
            .iter()
            .filter(|x| x.has_safe_fix())
            .map(|x| x.name.clone())
            .collect()
    })
}

/// Get the cached set of unsafe rule names
pub fn unsafe_rules_set() -> &'static HashSet<String> {
    UNSAFE_RULES.get_or_init(|| {
        all_rules_and_safety()
            .iter()
            .filter(|x| x.has_unsafe_fix())
            .map(|x| x.name.clone())
            .collect()
    })
}

/// Get the cached set of no-fix rule names
pub fn nofix_rules_set() -> &'static HashSet<String> {
    NOFIX_RULES.get_or_init(|| {
        all_rules_and_safety()
            .iter()
            .filter(|x| x.has_no_fix())
            .map(|x| x.name.clone())
            .collect()
    })
}

pub fn all_safe_rules() -> Vec<String> {
    safe_rules_set().iter().cloned().collect()
}

pub fn all_unsafe_rules() -> Vec<String> {
    unsafe_rules_set().iter().cloned().collect()
}

pub fn all_nofix_rules() -> Vec<String> {
    nofix_rules_set().iter().cloned().collect()
}

pub fn all_rules_enabled_by_default() -> Vec<String> {
    all_rules_and_safety()
        .iter()
        .filter(|x| x.is_enabled_by_default())
        .map(|x| x.name.clone())
        .collect()
}
