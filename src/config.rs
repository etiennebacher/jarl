use std::collections::HashMap;

use crate::utils::parse_rules_cli;
use air_r_parser::RParserOptions;

#[derive(Clone)]
pub struct Config<'a> {
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
