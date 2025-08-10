use std::path::PathBuf;

use crate::{args::CliArgs, lints::all_rules_and_safety, rule_table::RuleTable};

#[derive(Clone)]
pub struct Config {
    /// Paths to files to lint.
    pub paths: Vec<PathBuf>,
    /// List of rules and whether they have an associated safe fix, passed by
    /// the user and/or recovered from the config file. Those will
    /// not necessarily all be used, for instance if we disable unsafe fixes.
    pub rules: RuleTable,
    /// List of rules to use. If we lint only, then this is equivalent to the
    /// field `rules`. If we apply fixes too, then this might be different from
    /// `rules` because it may filter out rules that have unsafe fixes.
    pub rules_to_apply: RuleTable,
    /// Did the user pass the --fix flag?
    pub should_fix: bool,
    /// Did the user pass the --unsafe-fixes flag?
    pub unsafe_fixes: bool,
}

pub fn build_config(args: &CliArgs, paths: Vec<PathBuf>) -> Config {
    let rules = parse_rules_cli(&args.rules);
    let rules_to_apply: RuleTable = if args.fix && !args.unsafe_fixes {
        rules
            .iter()
            .filter(|r| r.should_fix)
            .cloned()
            .collect::<RuleTable>()
    } else {
        rules.clone()
    };

    Config {
        paths,
        rules,
        rules_to_apply,
        should_fix: args.fix,
        unsafe_fixes: args.unsafe_fixes,
    }
}

pub fn parse_rules_cli(rules: &str) -> RuleTable {
    if rules.is_empty() {
        all_rules_and_safety()
    } else {
        let passed_by_user = rules.split(",").collect::<Vec<&str>>();
        all_rules_and_safety()
            .iter()
            .filter(|r| passed_by_user.contains(&r.name.as_str()))
            .cloned()
            .collect::<RuleTable>()
    }
}
