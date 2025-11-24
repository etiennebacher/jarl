use crate::rule_table::{FixStatus, RuleTable};
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
pub(crate) mod expect_true_false;
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
pub(crate) mod system_file;
pub(crate) mod true_false_symbol;
pub(crate) mod which_grepl;

pub static RULE_GROUPS: &[&str] = &["CORR", "PERF", "READ", "SUSP", "TESTTHAT"];

/// Rule groups that are selected by default when no selectors are provided
pub static DEFAULT_SELECTORS: &[&str] = &["CORR", "PERF", "READ", "SUSP"];

/// List of supported rules and whether they have a safe fix.
///
/// Possible categories:
/// - CORR: correctness, code that is outright wrong or useless
/// - SUSP: suspicious, code that is most likely wrong or useless
/// - PERF: performance, code that can be written to run faster
/// - READ: readibility, code is correct but can be written in a way that is
///   easier to read.
pub fn all_rules_and_safety() -> RuleTable {
    let mut rule_table = RuleTable::empty();
    rule_table.enable("all_equal", "SUSP", FixStatus::Unsafe, None);
    rule_table.enable("any_duplicated", "PERF", FixStatus::Safe, None);
    rule_table.enable("any_is_na", "PERF", FixStatus::Safe, None);
    rule_table.enable("assignment", "READ", FixStatus::Safe, None);
    rule_table.enable("browser", "CORR", FixStatus::Safe, None);
    rule_table.enable("class_equals", "SUSP", FixStatus::Safe, None);
    rule_table.enable("comparison_negation", "READ", FixStatus::Safe, None);
    rule_table.enable("coalesce", "READ", FixStatus::Safe, Some((4, 4, 0)));
    rule_table.enable("download_file", "SUSP", FixStatus::None, None);
    rule_table.enable("duplicated_arguments", "SUSP", FixStatus::None, None);
    rule_table.enable("empty_assignment", "READ", FixStatus::Safe, None);
    rule_table.enable("equals_na", "CORR", FixStatus::Safe, None);
    rule_table.enable("expect_true_false", "TESTTHAT", FixStatus::Safe, None);
    rule_table.enable("for_loop_index", "READ", FixStatus::None, None);
    rule_table.enable("grepv", "READ", FixStatus::Safe, Some((4, 5, 0)));
    rule_table.enable("implicit_assignment", "READ", FixStatus::None, None);
    rule_table.enable("is_numeric", "READ", FixStatus::Safe, None);
    rule_table.enable("length_levels", "READ", FixStatus::Safe, None);
    rule_table.enable("length_test", "CORR", FixStatus::Safe, None);
    rule_table.enable("lengths", "PERF,READ", FixStatus::Safe, None);
    rule_table.enable("list2df", "PERF,READ", FixStatus::Safe, Some((4, 0, 0)));
    rule_table.enable("matrix_apply", "PERF", FixStatus::Safe, None);
    rule_table.enable("numeric_leading_zero", "READ", FixStatus::Safe, None);
    rule_table.enable("outer_negation", "PERF,READ", FixStatus::Safe, None);
    rule_table.enable("redundant_equals", "READ", FixStatus::Safe, None);
    rule_table.enable("repeat", "READ", FixStatus::Safe, None);
    rule_table.enable("sample_int", "READ", FixStatus::Safe, None);
    rule_table.enable("seq", "SUSP", FixStatus::Safe, None);
    rule_table.enable("seq2", "SUSP", FixStatus::Safe, None);
    rule_table.enable("sort", "PERF,READ", FixStatus::Safe, None);
    rule_table.enable("true_false_symbol", "READ", FixStatus::None, None);
    rule_table.enable("which_grepl", "PERF,READ", FixStatus::Safe, None);
    rule_table.enable("system_file", "READ", FixStatus::Safe, None);
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
