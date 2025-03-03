pub(crate) mod any_duplicated;
pub(crate) mod any_is_na;
pub(crate) mod class_equals;
pub(crate) mod duplicated_arguments;
pub(crate) mod empty_assignment;
pub(crate) mod equal_assignment;
pub(crate) mod equals_na;
pub(crate) mod length_levels;
pub(crate) mod true_false_symbol;

pub const ALL_RULES: &[&str] = &[
    "any_duplicated",
    "any_is_na",
    "class_equals",
    "duplicated_arguments",
    "empty_assignment",
    "equal_assignment",
    "equals_na",
    "length_levels",
    "true_false_symbol",
];
