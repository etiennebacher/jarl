pub(crate) mod any_duplicated;
pub(crate) mod any_is_na;
pub(crate) mod class_equals;
pub(crate) mod equals_na;
pub(crate) mod true_false_symbol;

pub const ALL_RULES: &[&str] = &[
    "any_duplicated",
    "any_is_na",
    "class_equals",
    "equals_na",
    "true_false_symbol",
];
