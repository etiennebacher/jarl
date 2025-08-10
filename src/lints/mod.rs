use crate::rule_table::RuleTable;

pub(crate) mod any_duplicated;
pub(crate) mod any_is_na;
pub(crate) mod class_equals;
pub(crate) mod duplicated_arguments;
pub(crate) mod empty_assignment;
pub(crate) mod equal_assignment;
pub(crate) mod equals_na;
// pub(crate) mod expect_length;
pub(crate) mod length_levels;
pub(crate) mod length_test;
pub(crate) mod lengths;
pub(crate) mod redundant_equals;
pub(crate) mod true_false_symbol;
pub(crate) mod which_grepl;

/// List of supported rules and whether they have a safe fix.
pub fn all_rules_and_safety() -> RuleTable {
    let mut rule_table = RuleTable::empty();
    rule_table.enable("any_duplicated", true, None);
    rule_table.enable("any_is_na", true, None);
    rule_table.enable("class_equals", true, None);
    rule_table.enable("duplicated_arguments", true, None);
    rule_table.enable("empty_assignment", true, None);
    rule_table.enable("equal_assignment", true, None);
    rule_table.enable("equals_na", true, None);
    rule_table.enable("length_levels", true, None);
    rule_table.enable("length_test", true, None);
    rule_table.enable("lengths", true, None);
    rule_table.enable("redundant_equals", true, None);
    rule_table.enable("true_false_symbol", false, None);
    rule_table.enable("which_grepl", true, None);
    rule_table
}
