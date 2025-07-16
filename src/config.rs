use std::{collections::HashMap, path::PathBuf};

use crate::{args::CliArgs, lints::all_rules_and_safety, toml::parse_flir_toml};

#[derive(Clone)]
pub struct Config<'a> {
    pub paths: Vec<PathBuf>,
    /// List of rules and whether they have an associated safe fix, passed by
    /// the user and/or recovered from the config file. Those will
    /// not necessarily all be used, for instance if we disable unsafe fixes.
    pub rules: HashMap<&'a str, bool>,
    /// List of rules to use. If we lint only, then this is equivalent to the
    /// field `rules`. If we apply fixes too, then this might be different from
    /// `rules` because it may filter out rules that have unsafe fixes.
    pub rules_to_apply: Vec<&'a str>,
    /// Did the user pass the --fix flag?
    pub should_fix: bool,
    /// Did the user pass the --unsafe-fixes flag?
    pub unsafe_fixes: bool,
}

pub fn build_config(args: &CliArgs, paths: Vec<PathBuf>) -> Config {
    let toml = parse_flir_toml(&PathBuf::from(&args.dir));
    let rules_toml = match toml {
        Ok(toml_options) => toml_options.linter.unwrap().rules.unwrap(),
        Err(_) => vec![],
    };

    let rules = parse_rules(&args.rules, rules_toml);
    let rules_to_apply: Vec<&str> = if args.fix && !args.unsafe_fixes {
        rules
            .iter()
            .filter(|(_, v)| **v)
            .map(|(k, _)| *k)
            .collect::<Vec<&str>>()
    } else {
        rules.keys().copied().collect()
    };

    Config {
        paths,
        rules,
        rules_to_apply,
        should_fix: args.fix,
        unsafe_fixes: args.unsafe_fixes,
    }
}

pub fn parse_rules(rules_cli: &str, rules_toml: Vec<String>) -> HashMap<&'static str, bool> {
    if rules_cli.is_empty() && rules_toml.is_empty() {
        return all_rules_and_safety();
    }
    let mut passed_by_user = rules_cli
        .split(",")
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();
    passed_by_user.extend(rules_toml);
    all_rules_and_safety()
        .iter()
        .filter(|(k, _)| passed_by_user.iter().any(|rule| rule == *k))
        .map(|(k, v)| (*k, *v))
        .collect::<HashMap<&'static str, bool>>()
}
