use crate::{args::CliArgs, lints::all_rules_and_safety, rule_table::RuleTable};
use anyhow::Result;
use std::path::PathBuf;

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
    pub apply_fixes: bool,
    /// Did the user pass the --unsafe-fixes flag?
    pub apply_unsafe_fixes: bool,
    /// The minimum R version used in the project. Used to disable some rules
    /// that require functions that are not available in all R versions, e.g.
    /// grepv() introduced in R 4.5.0.
    /// Since it's unlikely those functions are introduced in patch versions,
    /// this field takes only two numeric values.
    pub minimum_r_version: Option<(u32, u32)>,
}

pub fn build_config(args: &CliArgs, paths: Vec<PathBuf>) -> Result<Config> {
    let rules = parse_rules_cli(&args.rules);

    // Resolve the interaction between --fix and --unsafe-fixes first. Using
    // --unsafe-fixes implies using --fix, but the opposite is not true.
    let rules_to_apply = match (args.fix, args.unsafe_fixes) {
        (false, false) => rules.clone(),

        (true, false) => rules
            .iter()
            .filter(|r| r.has_no_fix() || r.has_safe_fix())
            .cloned()
            .collect::<RuleTable>(),

        (_, true) => rules
            .iter()
            .filter(|r| r.has_no_fix() || r.has_safe_fix() || r.has_unsafe_fix())
            .cloned()
            .collect::<RuleTable>(),
    };

    // We can now drop rules that don't have any fix if the user passed
    // --fix-only. This could maybe be done above but dealing with the three
    // args at the same time makes it much more complex.
    let rules_to_apply = if args.fix_only {
        rules
            .iter()
            .filter(|r| !r.has_no_fix())
            .cloned()
            .collect::<RuleTable>()
    } else {
        rules_to_apply
    };

    let minimum_r_version = parse_r_version(&args.min_r_version)?;

    Ok(Config {
        paths,
        rules,
        rules_to_apply,
        apply_fixes: args.fix,
        apply_unsafe_fixes: args.unsafe_fixes,
        minimum_r_version,
    })
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

pub fn parse_r_version(min_r_version: &Option<String>) -> Result<Option<(u32, u32)>> {
    if let Some(min_r_version) = min_r_version {
        // Check if the version contains exactly one dot and two parts
        if !min_r_version.contains('.') || min_r_version.split('.').count() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid version format. Expected 'x.y', e.g., '4.3'"
            ));
        }

        // Split by dot and try to parse each part as an integer
        let parts: Vec<&str> = min_r_version.split('.').collect();
        if let (Some(major), Some(minor)) = (parts.first(), parts.get(1)) {
            match (major.parse::<u32>(), minor.parse::<u32>()) {
                (Ok(major), Ok(minor)) => Ok(Some((major, minor))),
                _ => Err(anyhow::anyhow!("Version parts should be valid integers.")),
            }
        } else {
            Err(anyhow::anyhow!("Unexpected error in version parsing."))
        }
    } else {
        Ok(None)
    }
}
