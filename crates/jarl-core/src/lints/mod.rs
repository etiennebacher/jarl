use crate::rule_set::Rule;

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
pub(crate) mod redundant_ifelse;
pub(crate) mod repeat;
pub(crate) mod sample_int;
pub(crate) mod seq;
pub(crate) mod seq2;
pub(crate) mod sort;
pub(crate) mod sprintf;
pub(crate) mod string_boundary;
pub(crate) mod system_file;
pub(crate) mod true_false_symbol;
pub(crate) mod unnecessary_nesting;
pub(crate) mod unreachable_code;
pub(crate) mod vector_logic;
pub(crate) mod which_grepl;

/// Get all rules enabled by default
pub fn all_rules_enabled_by_default() -> Vec<String> {
    Rule::all()
        .iter()
        .filter(|r| r.is_enabled_by_default())
        .map(|r| r.name().to_string())
        .collect()
}
