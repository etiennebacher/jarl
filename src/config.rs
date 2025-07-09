use std::collections::HashMap;

use crate::lints::all_rules_and_safety;
use air_r_parser::RParserOptions;

#[derive(Clone)]
pub struct Config<'a> {
    /// List of rules and whether they have an associated safe fix, passed by
    /// the user and/or recovered from the config file. Those will
    /// not necessarily all be used, for instance if we disable unsafe fixes.
    pub rules: HashMap<&'a str, bool>,
    /// List of rules to use. If we lint only, then this is equivalent to the
    /// field `rules`. If we apply fixes too, then this might be different from
    /// `rules` because it may filter out rules that have unsafe fixes.
    pub rules_to_apply: Vec<&'a str>,
    pub should_fix: bool,
    pub allow_unsafe_fixes: bool,
    pub parser_options: RParserOptions,
}

pub fn build_config(
    rules_cli: &str,
    should_fix: bool,
    allow_unsafe_fixes: bool,
    parser_options: RParserOptions,
) -> Config {
    let rules = parse_rules_cli(rules_cli);
    let rules_to_apply: Vec<&str> = if should_fix && !allow_unsafe_fixes {
        rules
            .iter()
            .filter(|(_, v)| **v)
            .map(|(k, _)| *k)
            .collect::<Vec<&str>>()
    } else {
        rules.keys().map(|k| *k).collect()
    };

    Config {
        rules,
        rules_to_apply,
        should_fix,
        allow_unsafe_fixes,
        parser_options,
    }
}

pub fn parse_rules_cli(rules: &str) -> HashMap<&'static str, bool> {
    if rules.is_empty() {
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
