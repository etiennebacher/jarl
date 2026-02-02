use crate::rule_set::Rule;

pub(crate) mod base;
pub(crate) mod comments;
pub(crate) mod testthat;

/// Get all rules enabled by default
pub fn all_rules_enabled_by_default() -> Vec<String> {
    Rule::all()
        .iter()
        .filter(|r| r.is_enabled_by_default())
        .map(|r| r.name().to_string())
        .collect()
}
