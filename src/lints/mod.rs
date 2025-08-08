use std::collections::HashMap;

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
pub fn all_rules_and_safety() -> HashMap<&'static str, bool> {
    HashMap::from([
        ("any_duplicated", true),
        ("any_is_na", true),
        ("class_equals", true),
        ("duplicated_arguments", true),
        ("empty_assignment", true),
        ("equal_assignment", true),
        ("equals_na", true),
        // ("expect_length", false),
        ("length_levels", true),
        ("length_test", true),
        ("lengths", true),
        ("redundant_equals", true),
        ("true_false_symbol", false),
        ("which_grepl", true),
    ])
}
