use crate::rule_table::{FixStatus, RuleTable};

pub(crate) mod any_duplicated;
pub(crate) mod any_is_na;
pub(crate) mod assignment;
pub(crate) mod class_equals;
pub(crate) mod duplicated_arguments;
pub(crate) mod empty_assignment;
pub(crate) mod equals_na;
pub(crate) mod for_loop_index;
pub(crate) mod grepv;
pub(crate) mod implicit_assignment;
pub(crate) mod is_numeric;
pub(crate) mod length_levels;
pub(crate) mod length_test;
pub(crate) mod lengths;
pub(crate) mod matrix_apply;
pub(crate) mod numeric_leading_zero;
pub(crate) mod redundant_equals;
pub(crate) mod sample_int;
pub(crate) mod sort;
pub(crate) mod true_false_symbol;
pub(crate) mod which_grepl;

/// List of supported rules and whether they have a safe fix.
///
/// Possible categories:
/// * CORR: correctness, code that is outright wrong or useless
/// * SUSP: suspicious, code that is most likely wrong or useless
/// * PERF: performance, code that can be written to run faster
/// * READ: readibility, code is correct but can be written in a way that is
///         easier to read.
pub fn all_rules_and_safety() -> RuleTable {
    let mut rule_table = RuleTable::empty();
    rule_table.enable("any_duplicated", "PERF", FixStatus::Safe, None);
    rule_table.enable("any_is_na", "PERF", FixStatus::Safe, None);
    rule_table.enable("assignment", "READ", FixStatus::Safe, None);
    rule_table.enable("class_equals", "SUSP", FixStatus::Unsafe, None);
    rule_table.enable("duplicated_arguments", "SUSP", FixStatus::None, None);
    rule_table.enable("empty_assignment", "READ", FixStatus::Safe, None);
    rule_table.enable("equals_na", "CORR", FixStatus::Safe, None);
    rule_table.enable("for_loop_index", "READ", FixStatus::None, None);
    rule_table.enable("grepv", "READ", FixStatus::Safe, Some((4, 5, 0)));
    rule_table.enable("implicit_assignment", "READ", FixStatus::None, None);
    rule_table.enable("is_numeric", "READ", FixStatus::Safe, None);
    rule_table.enable("length_levels", "PERF,READ", FixStatus::Safe, None);
    rule_table.enable("length_test", "CORR", FixStatus::Safe, None);
    rule_table.enable("lengths", "PERF,READ", FixStatus::Safe, None);
    rule_table.enable("matrix_apply", "PERF", FixStatus::Safe, None);
    rule_table.enable("numeric_leading_zero", "READ", FixStatus::Safe, None);
    rule_table.enable("redundant_equals", "READ", FixStatus::Safe, None);
    rule_table.enable("sample_int", "READ", FixStatus::Safe, None);
    rule_table.enable("sort", "PERF,READ", FixStatus::Safe, None);
    rule_table.enable("true_false_symbol", "READ", FixStatus::None, None);
    rule_table.enable("which_grepl", "PERF,READ", FixStatus::Safe, None);
    rule_table
}

pub fn all_safe_rules() -> Vec<String> {
    all_rules_and_safety()
        .iter()
        .filter(|x| x.has_safe_fix())
        .map(|x| x.name.clone())
        .collect::<Vec<String>>()
}

pub fn all_unsafe_rules() -> Vec<String> {
    all_rules_and_safety()
        .iter()
        .filter(|x| x.has_unsafe_fix())
        .map(|x| x.name.clone())
        .collect::<Vec<String>>()
}

pub fn all_nofix_rules() -> Vec<String> {
    all_rules_and_safety()
        .iter()
        .filter(|x| x.has_no_fix())
        .map(|x| x.name.clone())
        .collect::<Vec<String>>()
}
