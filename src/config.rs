use std::collections::HashMap;

use crate::lints::all_rules_and_safety;
use air_r_parser::RParserOptions;

#[derive(Clone)]
pub struct Config<'a> {
    /// List of rules to use and whether they have an associated safe fix
    pub rules: HashMap<&'a str, bool>,
    pub should_fix: bool,
    pub unsafe_fixes: bool,
    pub parser_options: RParserOptions,
}

pub fn build_config(
    rules_cli: &str,
    should_fix: bool,
    unsafe_fixes: bool,
    parser_options: RParserOptions,
) -> Config {
    let rules = parse_rules_cli(rules_cli);
    Config { rules, should_fix, unsafe_fixes, parser_options }
}

pub fn parse_rules_cli(rules: &str) -> HashMap<&'static str, bool> {
    if rules == "" {
        all_rules_and_safety()
    } else {
        let passed_by_user = rules.split(",").collect::<Vec<&str>>();
        all_rules_and_safety()
            .iter()
            .filter(|(k, _)| passed_by_user.contains(*k))
            .map(|(k, v)| (*k, *v))
            .collect::<HashMap<&'static str, bool>>()
    }
}
