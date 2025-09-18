use crate::{
    args::CliArgs, description::Description, lints::all_rules_and_safety, rule_table::RuleTable,
};
use anyhow::Result;
use std::{collections::HashSet, fs, path::PathBuf};

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
    pub minimum_r_version: Option<(u32, u32, u32)>,
}

pub fn build_config(args: &CliArgs, paths: Vec<PathBuf>) -> Result<Config> {
    // Determining the minimum R version has to come first since if it is
    // unknown then only rules that don't have a version restriction are
    // selected.
    let minimum_r_version = determine_minimum_r_version(args, &paths)?;

    let rules = parse_rules_cli(&args.select_rules, &args.ignore_rules)?;

    let rules = filter_rules_by_version(&rules, minimum_r_version);

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

    Ok(Config {
        paths,
        rules,
        rules_to_apply,
        apply_fixes: args.fix,
        apply_unsafe_fixes: args.unsafe_fixes,
        minimum_r_version,
    })
}

/// Resolve the rules to use, based on the CLI arguments only.
///
/// If `--select-rules` is not passed by the user, then we use all existing
/// rules.
/// If `--ignore-rules` is not passed by the user, then we don't exclude any
/// rules.
///
/// `--ignore-rules` always has the last word: if a rule is in both
/// `--select-rules` and `--ignore-rules`, then it is ignored.
pub fn parse_rules_cli(select_rules: &str, ignore_rules: &str) -> Result<RuleTable> {
    let all_rules = all_rules_and_safety();

    let selected_rules: HashSet<String> = if select_rules.is_empty() {
        HashSet::from_iter(all_rules.iter().map(|x| x.name.clone()))
    } else {
        let passed_by_user = select_rules.split(",").collect::<Vec<&str>>();
        let invalid_rules = get_invalid_rules(&all_rules, &passed_by_user);
        if let Some(invalid_rules) = invalid_rules {
            return Err(anyhow::anyhow!(
                "Unknown rules in `--select-rules`: {}",
                invalid_rules.join(", ")
            ));
        }

        HashSet::from_iter(
            all_rules
                .iter()
                .filter(|r| passed_by_user.contains(&r.name.as_str()))
                .map(|x| x.name.clone()),
        )
    };

    let ignored_rules: HashSet<String> = if ignore_rules.is_empty() {
        HashSet::new()
    } else {
        let passed_by_user = ignore_rules.split(",").collect::<Vec<&str>>();
        let invalid_rules = get_invalid_rules(&all_rules, &passed_by_user);
        if let Some(invalid_rules) = invalid_rules {
            return Err(anyhow::anyhow!(
                "Unknown rules in `--ignore-rules`: {}",
                invalid_rules.join(", ")
            ));
        }

        HashSet::from_iter(
            all_rules
                .iter()
                .filter(|r| passed_by_user.contains(&r.name.as_str()))
                .map(|x| x.name.clone()),
        )
    };

    let final_rule_names: HashSet<String> =
        selected_rules.difference(&ignored_rules).cloned().collect();

    let final_rules: RuleTable = all_rules
        .iter()
        .filter(|r| final_rule_names.contains(&r.name))
        .cloned()
        .collect();

    Ok(final_rules)
}

/// Determine the minimum R version from CLI args or DESCRIPTION file
fn determine_minimum_r_version(
    args: &CliArgs,
    paths: &[PathBuf],
) -> Result<Option<(u32, u32, u32)>> {
    if let Some(version_str) = &args.min_r_version {
        return Ok(Some(parse_r_version(version_str.clone())?));
    }

    // Look for DESCRIPTION file in any of the project paths
    // TODO: this seems wasteful but I don't have a good infrastructure for now
    // for getting the common root of the paths.
    for path in paths {
        let desc_path = if path.is_dir() {
            path.join("DESCRIPTION")
        } else if let Some(parent) = path.parent() {
            parent.join("DESCRIPTION")
        } else {
            continue;
        };

        if desc_path.exists() {
            let desc = fs::read_to_string(&desc_path)?;
            if let Ok(versions) = Description::get_depend_r_version(&desc) {
                if let Some(version_str) = versions.first() {
                    return Ok(Some(parse_r_version(version_str.to_string())?));
                }
            }
        }
    }

    Ok(None)
}

/// Parse R version string in format "x.y" or "x.y.z" and return (major, minor, patch)
pub fn parse_r_version(min_r_version: String) -> Result<(u32, u32, u32)> {
    let parts: Vec<&str> = min_r_version.split('.').collect();

    if parts.len() < 2 || parts.len() > 3 {
        return Err(anyhow::anyhow!(
            "Invalid version format. Expected 'x.y' or 'x.y.z', e.g., '4.3' or '4.3.0'"
        ));
    }

    let major = parts[0]
        .parse::<u32>()
        .map_err(|_| anyhow::anyhow!("Major version should be a valid integer"))?;
    let minor = parts[1]
        .parse::<u32>()
        .map_err(|_| anyhow::anyhow!("Minor version should be a valid integer"))?;
    let patch = if parts.len() == 3 {
        parts[2]
            .parse::<u32>()
            .map_err(|_| anyhow::anyhow!("Patch version should be a valid integer"))?
    } else {
        0
    };

    Ok((major, minor, patch))
}

/// Filter rules based on minimum R version compatibility
fn filter_rules_by_version(
    rules: &RuleTable,
    minimum_r_version: Option<(u32, u32, u32)>,
) -> RuleTable {
    match minimum_r_version {
        None => {
            // If we don't know the minimum R version, only include rules without version requirements
            rules
                .iter()
                .filter(|rule| rule.minimum_r_version.is_none())
                .cloned()
                .collect::<RuleTable>()
        }
        Some(project_min_version) => {
            // Include rules that are compatible with the minimum version
            rules
                .iter()
                .filter(|rule| {
                    match rule.minimum_r_version {
                        None => true, // Rule has no version requirement
                        Some(rule_min_version) => {
                            // For instance, grepv() exists only for R >= 4.5.0,
                            // so we enable it only if the project version is
                            // guaranteed to be above this rule version.
                            rule_min_version <= project_min_version
                        }
                    }
                })
                .cloned()
                .collect::<RuleTable>()
        }
    }
}

fn get_invalid_rules(
    all_rule_names: &RuleTable,
    rules_passed_by_user: &Vec<&str>,
) -> Option<Vec<String>> {
    let all_rules_set: HashSet<_> = all_rule_names.iter().map(|x| x.name.clone()).collect();

    let invalid_rules: Vec<String> = rules_passed_by_user
        .into_iter()
        .filter(|rule| !all_rules_set.contains(&rule.to_string()))
        .map(|x| x.to_string())
        .collect();

    if invalid_rules.is_empty() {
        None
    } else {
        Some(invalid_rules)
    }
}
