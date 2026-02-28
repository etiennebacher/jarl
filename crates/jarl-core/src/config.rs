use crate::{
    description::Description,
    lints::all_rules_enabled_by_default,
    rule_options::ResolvedRuleOptions,
    rule_set::{Category, Rule, RuleSet},
    settings::Settings,
};
use air_r_syntax::RSyntaxKind;
use anyhow::Result;
use std::{collections::HashSet, fs, path::PathBuf};

use crate::rule_options::assignment::ResolvedAssignmentOptions;

/// Parsed rule selection from CLI or TOML configuration.
/// Contains selected rules, extended rules, and ignored rules.
#[derive(Debug)]
pub struct RuleSelection {
    pub selected: Option<HashSet<String>>,
    pub extended: Option<HashSet<String>>,
    pub ignored: HashSet<String>,
}

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
    pub select: String,
    /// Additional rules to add to the selection. A single string with commas between rule names.
    pub extend_select: String,
    /// Names of rules to ignore. A single string with commas between rule names.
    pub ignore: String,
    /// The minimum R version used in the project. Used to disable some rules
    /// that require functions that are not available in all R versions, e.g.
    /// grepv() introduced in R 4.5.0.
    pub min_r_version: Option<String>,
    /// Apply fixes even if the Git branch still has uncommitted files?
    pub allow_dirty: bool,
    /// Apply fixes even if there is no version control system?
    pub allow_no_vcs: bool,
    /// Which assignment operator to use? Can be `"<-"` or `"="`.
    pub assignment: Option<String>,
}

#[derive(Clone)]
pub struct Config {
    /// Paths to files to lint.
    pub paths: Vec<PathBuf>,
    /// List of rules and whether they have an associated safe fix, passed by
    /// the user and/or recovered from the config file. Those will
    /// not necessarily all be used, for instance if we disable unsafe fixes.
    pub rules: RuleSet,
    /// List of rules to use. If we lint only, then this is equivalent to the
    /// field `rules`. If we apply fixes too, then this might be different from
    /// `rules` because it may filter out rules that have unsafe fixes.
    pub rules_to_apply: RuleSet,
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
    /// Rules that should not have their fixes applied (from unfixable setting)
    pub unfixable: HashSet<String>,
    /// Rules that are allowed to have fixes applied (from fixable setting)
    /// None means all rules with fixes can be applied
    pub fixable: Option<HashSet<String>>,
    /// Resolved per-rule options
    pub rule_options: ResolvedRuleOptions,
}

pub fn build_config(
    check_config: &ArgsConfig,
    toml_settings: Option<&Settings>,
    paths: Vec<PathBuf>,
) -> Result<Config> {
    // Determining the minimum R version has to come first since if it is
    // unknown then only rules that don't have a version restriction are
    // selected.
    let minimum_r_version = determine_minimum_r_version(check_config, &paths)?;

    let rules_cli = parse_rules_cli(
        &check_config.select,
        &check_config.extend_select,
        &check_config.ignore,
    )?;
    let rules_toml = parse_rules_toml(toml_settings)?;
    let rules = reconcile_rules(rules_cli, rules_toml)?;

    let rules = filter_rules_by_version(&rules, minimum_r_version);

    // Parse fixable/unfixable rules from TOML.
    // These will be stored in Config and checked when applying fixes.
    let (fixable_toml, unfixable_toml) = parse_fixable_toml(toml_settings)?;

    // Resolve the interaction between --fix and --unsafe-fixes first. Using
    // --unsafe-fixes implies using --fix, but the opposite is not true.
    let rules_to_apply = match (check_config.fix, check_config.unsafe_fixes) {
        (false, false) => rules.clone(),

        (true, false) => rules
            .iter()
            .filter(|r| r.has_no_fix() || r.has_safe_fix())
            .collect::<RuleSet>(),

        (_, true) => rules
            .iter()
            .filter(|r| r.has_no_fix() || r.has_safe_fix() || r.has_unsafe_fix())
            .collect::<RuleSet>(),
    };

    // We can now drop rules that don't have any fix if the user passed
    // --fix-only. This could maybe be done above but dealing with the three
    // args at the same time makes it much more complex.
    let rules_to_apply = if check_config.fix_only {
        rules
            .iter()
            .filter(|r| !r.has_no_fix())
            .collect::<RuleSet>()
    } else {
        rules_to_apply
    };

    let mut rule_options = toml_settings
        .map(|s| s.linter.rule_options.clone())
        .unwrap_or_default();

    // CLI --assignment overrides the TOML-resolved value
    if let Some(cli_assignment) = &check_config.assignment {
        rule_options.assignment = parse_assignment_cli(cli_assignment)?;
    }

    Ok(Config {
        paths,
        rules,
        rules_to_apply,
        apply_fixes: check_config.fix,
        apply_unsafe_fixes: check_config.unsafe_fixes,
        minimum_r_version,
        allow_dirty: check_config.allow_dirty,
        allow_no_vcs: check_config.allow_no_vcs,
        unfixable: unfixable_toml,
        fixable: fixable_toml,
        rule_options,
    })
}

/// Parse CLI rule arguments and return (selected_rules, ignored_rules).
///
/// Returns None for selected_rules if no --select was specified.
/// Returns empty set for ignored_rules if no --ignore was specified.
pub fn parse_rules_cli(select: &str, extend_select: &str, ignore: &str) -> Result<RuleSelection> {
    let all_rules = Rule::all();

    let selected_rules: Option<HashSet<String>> = if select.is_empty() {
        None
    } else {
        let passed_by_user = select.split(",").collect::<Vec<&str>>();
        let expanded_rules = replace_group_rules(&passed_by_user, all_rules);
        let invalid_rules = get_invalid_rules(all_rules, &expanded_rules);
        if let Some(invalid_rules) = invalid_rules {
            return Err(anyhow::anyhow!(
                "Unknown rules in `--select`: {}",
                invalid_rules.join(", ")
            ));
        }

        Some(HashSet::from_iter(
            all_rules
                .iter()
                .filter(|r| expanded_rules.iter().any(|name| name == r.name()))
                .map(|x| x.name().to_string()),
        ))
    };

    let extended_rules: Option<HashSet<String>> = if extend_select.is_empty() {
        None
    } else {
        let passed_by_user = extend_select.split(",").collect::<Vec<&str>>();
        let expanded_rules = replace_group_rules(&passed_by_user, all_rules);
        let invalid_rules = get_invalid_rules(all_rules, &expanded_rules);
        if let Some(invalid_rules) = invalid_rules {
            return Err(anyhow::anyhow!(
                "Unknown rules in `--extend-select`: {}",
                invalid_rules.join(", ")
            ));
        }

        Some(HashSet::from_iter(
            all_rules
                .iter()
                .filter(|r| expanded_rules.iter().any(|name| name == r.name()))
                .map(|x| x.name().to_string()),
        ))
    };

    let ignored_rules: HashSet<String> = if ignore.is_empty() {
        HashSet::new()
    } else {
        let passed_by_user = ignore.split(",").collect::<Vec<&str>>();
        let expanded_rules = replace_group_rules(&passed_by_user, all_rules);
        let invalid_rules = get_invalid_rules(all_rules, &expanded_rules);
        if let Some(invalid_rules) = invalid_rules {
            return Err(anyhow::anyhow!(
                "Unknown rules in `--ignore`: {}",
                invalid_rules.join(", ")
            ));
        }

        HashSet::from_iter(
            all_rules
                .iter()
                .filter(|r| expanded_rules.iter().any(|name| name == r.name()))
                .map(|x| x.name().to_string()),
        )
    };

    Ok(RuleSelection {
        selected: selected_rules,
        extended: extended_rules,
        ignored: ignored_rules,
    })
}

/// Parse TOML configuration and return (selected_rules, ignored_rules).
///
/// Returns None for selected_rules if no TOML select was specified (meaning use all rules).
/// Returns empty set for ignored_rules if no TOML ignore was specified.
pub fn parse_rules_toml(toml_settings: Option<&Settings>) -> Result<RuleSelection> {
    let all_rules = Rule::all();

    let Some(settings) = toml_settings else {
        // No TOML configuration found
        return Ok(RuleSelection {
            selected: None,
            extended: None,
            ignored: HashSet::new(),
        });
    };

    let linter_settings = &settings.linter;

    // Handle select rules from TOML
    let selected_rules: Option<HashSet<String>> = if let Some(select) = &linter_settings.select {
        let passed_by_user = select.iter().map(|s| s.as_str()).collect();
        let expanded_rules = replace_group_rules(&passed_by_user, all_rules);
        let invalid_rules = get_invalid_rules(all_rules, &expanded_rules);
        if let Some(invalid_rules) = invalid_rules {
            return Err(anyhow::anyhow!(
                "Unknown rules in field `select` in 'jarl.toml': {}",
                invalid_rules.join(", ")
            ));
        }
        Some(HashSet::from_iter(
            all_rules
                .iter()
                .filter(|r| expanded_rules.iter().any(|name| name == r.name()))
                .map(|x| x.name().to_string()),
        ))
    } else {
        None
    };

    // Handle extend-select rules from TOML
    let extended_rules: Option<HashSet<String>> =
        if let Some(extend_select) = &linter_settings.extend_select {
            let passed_by_user = extend_select.iter().map(|s| s.as_str()).collect();
            let expanded_rules = replace_group_rules(&passed_by_user, all_rules);
            let invalid_rules = get_invalid_rules(all_rules, &expanded_rules);
            if let Some(invalid_rules) = invalid_rules {
                return Err(anyhow::anyhow!(
                    "Unknown rules in field `extend-select` in 'jarl.toml': {}",
                    invalid_rules.join(", ")
                ));
            }
            Some(HashSet::from_iter(
                all_rules
                    .iter()
                    .filter(|r| expanded_rules.iter().any(|name| name == r.name()))
                    .map(|x| x.name().to_string()),
            ))
        } else {
            None
        };

    // Handle ignore rules from TOML
    let ignored_rules: HashSet<String> = if let Some(ignore) = &linter_settings.ignore {
        let passed_by_user = ignore.iter().map(|s| s.as_str()).collect();
        let expanded_rules = replace_group_rules(&passed_by_user, all_rules);
        let invalid_rules = get_invalid_rules(all_rules, &expanded_rules);
        if let Some(invalid_rules) = invalid_rules {
            return Err(anyhow::anyhow!(
                "Unknown rules in field `ignore` in 'jarl.toml': {}",
                invalid_rules.join(", ")
            ));
        }
        HashSet::from_iter(
            all_rules
                .iter()
                .filter(|r| expanded_rules.iter().any(|name| name == r.name()))
                .map(|x| x.name().to_string()),
        )
    } else {
        HashSet::new()
    };

    Ok(RuleSelection {
        selected: selected_rules,
        extended: extended_rules,
        ignored: ignored_rules,
    })
}

/// Parse fixable and unfixable rules from TOML configuration.
///
/// Returns (fixable_rules, unfixable_rules).
/// Returns None for fixable_rules if no fixable was specified in TOML.
/// Returns empty set for unfixable_rules if no unfixable was specified in TOML.
pub fn parse_fixable_toml(
    toml_settings: Option<&Settings>,
) -> Result<(Option<HashSet<String>>, HashSet<String>)> {
    let all_rules = Rule::all();

    let Some(settings) = toml_settings else {
        // No TOML configuration found
        return Ok((None, HashSet::new()));
    };

    let linter_settings = &settings.linter;

    // Handle fixable rules from TOML
    let fixable_rules: Option<HashSet<String>> =
        if let Some(fixable_rules) = &linter_settings.fixable {
            let passed_by_user = fixable_rules.iter().map(|s| s.as_str()).collect();
            let expanded_rules = replace_group_rules(&passed_by_user, all_rules);
            let invalid_rules = get_invalid_rules(all_rules, &expanded_rules);
            if let Some(invalid_rules) = invalid_rules {
                return Err(anyhow::anyhow!(
                    "Unknown rules in field `fixable` in 'jarl.toml': {}",
                    invalid_rules.join(", ")
                ));
            }
            Some(HashSet::from_iter(
                all_rules
                    .iter()
                    .filter(|r| expanded_rules.iter().any(|name| name == r.name()))
                    .map(|x| x.name().to_string()),
            ))
        } else {
            None
        };

    // Handle unfixable rules from TOML
    let unfixable_rules: HashSet<String> = if let Some(unfixable_rules) = &linter_settings.unfixable
    {
        let passed_by_user = unfixable_rules.iter().map(|s| s.as_str()).collect();
        let expanded_rules = replace_group_rules(&passed_by_user, all_rules);
        let invalid_rules = get_invalid_rules(all_rules, &expanded_rules);
        if let Some(invalid_rules) = invalid_rules {
            return Err(anyhow::anyhow!(
                "Unknown rules in field `unfixable` in 'jarl.toml': {}",
                invalid_rules.join(", ")
            ));
        }
        HashSet::from_iter(
            all_rules
                .iter()
                .filter(|r| expanded_rules.iter().any(|name| name == r.name()))
                .map(|x| x.name().to_string()),
        )
    } else {
        HashSet::new()
    };

    Ok((fixable_rules, unfixable_rules))
}

// This takes rules that refer to groups (e.g. "PERF", "READ") and replaces them
// with the rule names.
// Returns a vector with the original rule names left unmodified and the expanded
// group names.
fn replace_group_rules(rules_passed_by_user: &Vec<&str>, all_rules: &[Rule]) -> Vec<String> {
    let rule_groups_set: HashSet<&str> = Category::ALL.iter().map(|c| c.as_str()).collect();
    let mut expanded_rules = Vec::new();

    for &rule_or_group in rules_passed_by_user {
        let trimmed = rule_or_group.trim();

        if trimmed == "ALL" {
            // Special keyword to select all rules (including opt-in ones)
            for rule in all_rules.iter() {
                expanded_rules.push(rule.name().to_string());
            }
        } else if rule_groups_set.contains(trimmed) {
            // This is a group name, expand it to all rules in that group
            if let Ok(category) = trimmed.parse::<Category>() {
                for rule in all_rules.iter() {
                    if rule.has_category(category) {
                        expanded_rules.push(rule.name().to_string());
                    }
                }
            }
        } else {
            // This is a rule name (or invalid input), keep as-is
            expanded_rules.push(trimmed.to_string());
        }
    }
    expanded_rules
}

// This finds invalid rule names and throws an error with their names in the
// message.
//
// It is important this comes after expanding group names (e.g. "PERF") to
// individual rule names.
fn get_invalid_rules(
    all_rule_names: &[Rule],
    rules_passed_by_user: &[String],
) -> Option<Vec<String>> {
    let all_rules_set: HashSet<_> = all_rule_names.iter().map(|x| x.name()).collect();

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
                x.clone()
            }
        })
        .collect();

    if invalid_rules.is_empty() {
        None
    } else {
        Some(invalid_rules)
    }
}

/// Reconcile rules from CLI and TOML configuration.
///
/// Strategy:
/// - CLI select takes precedence over TOML select
/// - CLI ignore and TOML ignore are combined (both applied)
/// - If neither CLI nor TOML specify select, start with all rules
fn reconcile_rules(rules_cli: RuleSelection, rules_toml: RuleSelection) -> Result<RuleSet> {
    let all_rules = Rule::all();
    let cli_selected = rules_cli.selected;
    let cli_extended = rules_cli.extended;
    let cli_ignored = rules_cli.ignored;
    let toml_selected = rules_toml.selected;
    let toml_extended = rules_toml.extended;
    let toml_ignored = rules_toml.ignored;

    // Step 1: Determine base selection (CLI select takes precedence over TOML select)
    let base_selected: HashSet<String> = if let Some(cli_selected) = cli_selected {
        // CLI select specified, use it
        cli_selected
    } else if let Some(toml_selected) = toml_selected {
        // No CLI select, but TOML select exists, use TOML
        toml_selected
    } else {
        // Neither CLI nor TOML specified select rules, use the default set of rules
        HashSet::from_iter(all_rules_enabled_by_default())
    };

    // Step 2: Add extended rules (CLI extend-select takes precedence over TOML extend-select)
    let mut final_selected = base_selected;
    if let Some(cli_extended) = cli_extended {
        final_selected = final_selected.union(&cli_extended).cloned().collect();
    } else if let Some(toml_extended) = toml_extended {
        final_selected = final_selected.union(&toml_extended).cloned().collect();
    }

    // Step 3: Combine all ignore rules (TOML + CLI)
    let all_ignored: HashSet<String> = cli_ignored.union(&toml_ignored).cloned().collect();

    // Step 4: Apply ignore rules to final selection
    let final_rule_names: HashSet<String> =
        final_selected.difference(&all_ignored).cloned().collect();

    let final_rules: RuleSet = all_rules
        .iter()
        .filter(|r| final_rule_names.iter().any(|name| name == r.name()))
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
            if let Ok(versions) = Description::get_depend_r_version(&desc)
                && let Some(version_str) = versions.first()
            {
                return Ok(Some(parse_r_version(version_str.to_string())?));
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
fn filter_rules_by_version(rules: &RuleSet, minimum_r_version: Option<(u32, u32, u32)>) -> RuleSet {
    match minimum_r_version {
        None => {
            // If we don't know the minimum R version, only include rules without version requirements
            rules
                .iter()
                .filter(|rule| rule.minimum_r_version().is_none())
                .collect::<RuleSet>()
        }
        Some(project_min_version) => {
            // Include rules that are compatible with the minimum version
            // Only include rules that either have no version requirement or meet the minimum version
            rules
                .iter()
                .filter(|rule| {
                    match rule.minimum_r_version() {
                        None => true, // Rule has no version requirement
                        Some(rule_min_version) => {
                            // For instance, grepv() exists only for R >= 4.5.0,
                            // so we enable it only if the project version is
                            // guaranteed to be above this rule version.
                            rule_min_version <= project_min_version
                        }
                    }
                })
                .collect::<RuleSet>()
        }
    }
}

fn parse_assignment_cli(value: &str) -> Result<ResolvedAssignmentOptions> {
    match value {
        "<-" => Ok(ResolvedAssignmentOptions { operator: RSyntaxKind::ASSIGN }),
        "=" => Ok(ResolvedAssignmentOptions { operator: RSyntaxKind::EQUAL }),
        _ => Err(anyhow::anyhow!(
            "Invalid value in `--assignment`: {}",
            value
        )),
    }
}
