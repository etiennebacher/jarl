use crate::{
    description::Description, lints::all_rules_and_safety, rule_table::RuleTable,
    settings::Settings,
};
use air_workspace::resolve::PathResolver;
use anyhow::Result;
use std::{collections::HashSet, fs, path::PathBuf};

#[derive(Clone, Debug)]
/// Arguments provided in the CLI.
pub struct ArgsConfig {
    /// Paths to files to lint.
    pub files: Vec<PathBuf>,
    /// Did the user pass the --fix flag?
    pub fix: bool,
    /// Did the user pass the --unsafe-fixes flag?
    pub unsafe_fixes: bool,
    /// Did the user pass the --fix-only flag?
    pub fix_only: bool,
    /// Names of rules to use. A single string with commas between rule names.
    pub select_rules: String,
    /// Names of rules to ignore. A single string with commas between rule names.
    pub ignore_rules: String,
    /// The minimum R version used in the project. Used to disable some rules
    /// that require functions that are not available in all R versions, e.g.
    /// grepv() introduced in R 4.5.0.
    pub min_r_version: Option<String>,
    /// Apply fixes even if the Git branch still has uncommitted files?
    pub allow_dirty: bool,
    /// Apply fixes even if there is no version control system?
    pub allow_no_vcs: bool,
}

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
    /// Apply fixes even if the Git branch still has uncommitted files?
    pub allow_dirty: bool,
    /// Apply fixes even if there is no version control system?
    pub allow_no_vcs: bool,
}

pub fn build_config(
    check_config: &ArgsConfig,
    resolver: &PathResolver<Settings>,
    paths: Vec<PathBuf>,
) -> Result<Config> {
    let root_path = resolver
        .items()
        .iter()
        .map(|x| x.path())
        .collect::<Vec<_>>();

    if root_path.len() > 1 {
        todo!("Don't know how to handle multiple TOML")
    }

    let toml_settings = if root_path.len() == 1 {
        Some(resolver.items().first().unwrap().value())
    } else {
        None
    };

    // Determining the minimum R version has to come first since if it is
    // unknown then only rules that don't have a version restriction are
    // selected.
    let minimum_r_version = determine_minimum_r_version(check_config, &paths)?;

    let rules_cli = parse_rules_cli(&check_config.select_rules, &check_config.ignore_rules)?;
    let rules_toml = parse_rules_toml(toml_settings)?;
    let rules = reconcile_rules(rules_cli, rules_toml)?;

    let rules = filter_rules_by_version(&rules, minimum_r_version);

    // Resolve the interaction between --fix and --unsafe-fixes first. Using
    // --unsafe-fixes implies using --fix, but the opposite is not true.
    let rules_to_apply = match (check_config.fix, check_config.unsafe_fixes) {
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
    let rules_to_apply = if check_config.fix_only {
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
        apply_fixes: check_config.fix,
        apply_unsafe_fixes: check_config.unsafe_fixes,
        minimum_r_version,
        allow_dirty: check_config.allow_dirty,
        allow_no_vcs: check_config.allow_no_vcs,
    })
}

/// Parse CLI rule arguments and return (selected_rules, ignored_rules).
///
/// Returns None for selected_rules if no --select-rules was specified.
/// Returns empty set for ignored_rules if no --ignore-rules was specified.
pub fn parse_rules_cli(
    select_rules: &str,
    ignore_rules: &str,
) -> Result<(Option<HashSet<String>>, HashSet<String>)> {
    let all_rules = all_rules_and_safety();

    let selected_rules: Option<HashSet<String>> = if select_rules.is_empty() {
        None
    } else {
        let passed_by_user = select_rules.split(",").collect::<Vec<&str>>();
        let invalid_rules = get_invalid_rules(&all_rules, &passed_by_user);
        if let Some(invalid_rules) = invalid_rules {
            return Err(anyhow::anyhow!(
                "Unknown rules in `--select-rules`: {}",
                invalid_rules.join(", ")
            ));
        }

        Some(HashSet::from_iter(
            all_rules
                .iter()
                .filter(|r| passed_by_user.contains(&r.name.as_str()))
                .map(|x| x.name.clone()),
        ))
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

    Ok((selected_rules, ignored_rules))
}

/// Parse TOML configuration and return (selected_rules, ignored_rules).
///
/// Returns None for selected_rules if no TOML select was specified (meaning use all rules).
/// Returns empty set for ignored_rules if no TOML ignore was specified.
pub fn parse_rules_toml(
    toml_settings: Option<&Settings>,
) -> Result<(Option<HashSet<String>>, HashSet<String>)> {
    let all_rules = all_rules_and_safety();

    let Some(settings) = toml_settings else {
        // No TOML configuration found
        return Ok((None, HashSet::new()));
    };

    let linter_settings = &settings.linter;

    // Handle select rules from TOML
    let selected_rules: Option<HashSet<String>> =
        if let Some(select_rules) = &linter_settings.select {
            let invalid_rules = get_invalid_rules(
                &all_rules,
                &select_rules.iter().map(|s| s.as_str()).collect(),
            );
            if let Some(invalid_rules) = invalid_rules {
                return Err(anyhow::anyhow!(
                    "Unknown rules in field `select` in 'jarl.toml': {}",
                    invalid_rules.join(", ")
                ));
            }
            Some(HashSet::from_iter(select_rules.iter().cloned()))
        } else {
            None
        };

    // Handle ignore rules from TOML
    let ignored_rules: HashSet<String> = if let Some(ignore_rules) = &linter_settings.ignore {
        let invalid_rules = get_invalid_rules(
            &all_rules,
            &ignore_rules.iter().map(|s| s.as_str()).collect(),
        );
        if let Some(invalid_rules) = invalid_rules {
            return Err(anyhow::anyhow!(
                "Unknown rules in field `ignore` in 'jarl.toml': {}",
                invalid_rules.join(", ")
            ));
        }
        HashSet::from_iter(ignore_rules.iter().cloned())
    } else {
        HashSet::new()
    };

    Ok((selected_rules, ignored_rules))
}

/// Reconcile rules from CLI and TOML configuration.
///
/// Strategy:
/// - CLI select takes precedence over TOML select
/// - CLI ignore and TOML ignore are combined (both applied)
/// - If neither CLI nor TOML specify select, start with all rules
fn reconcile_rules(
    rules_cli: (Option<HashSet<String>>, HashSet<String>),
    rules_toml: (Option<HashSet<String>>, HashSet<String>),
) -> Result<RuleTable> {
    let all_rules = all_rules_and_safety();
    let (cli_selected, cli_ignored) = rules_cli;
    let (toml_selected, toml_ignored) = rules_toml;

    // Step 1: Determine base selection (CLI select takes precedence over TOML select)
    let base_selected: HashSet<String> = if let Some(cli_selected) = cli_selected {
        // CLI select specified, use it
        cli_selected
    } else if let Some(toml_selected) = toml_selected {
        // No CLI select, but TOML select exists, use TOML
        toml_selected
    } else {
        // Neither CLI nor TOML specified select rules, start with all rules
        HashSet::from_iter(all_rules.iter().map(|x| x.name.clone()))
    };

    // Step 2: Combine all ignore rules (TOML + CLI)
    let all_ignored: HashSet<String> = cli_ignored.union(&toml_ignored).cloned().collect();

    // Step 3: Apply ignore rules to base selection
    let final_rule_names: HashSet<String> =
        base_selected.difference(&all_ignored).cloned().collect();

    let final_rules: RuleTable = all_rules
        .iter()
        .filter(|r| final_rule_names.contains(&r.name))
        .cloned()
        .collect();

    Ok(final_rules)
}

/// Determine the minimum R version from CLI args or DESCRIPTION file
fn determine_minimum_r_version(
    check_config: &ArgsConfig,
    paths: &[PathBuf],
) -> Result<Option<(u32, u32, u32)>> {
    if let Some(version_string) = &check_config.min_r_version {
        return Ok(Some(parse_r_version(version_string.clone())?));
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
        .iter()
        .filter(|rule| {
            let trimmed = rule.trim();
            // Rule is invalid if it's empty/whitespace-only or doesn't exist in valid rules
            trimmed.is_empty() || !all_rules_set.contains(trimmed)
        })
        .map(|x| {
            let trimmed = x.trim();
            if trimmed.is_empty() {
                format!("\"{x}\" (empty or whitespace-only not allowed)")
            } else {
                x.to_string()
            }
        })
        .collect();

    if invalid_rules.is_empty() {
        None
    } else {
        Some(invalid_rules)
    }
}
