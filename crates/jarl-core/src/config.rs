use crate::{
    error::UnknownRulesError,
    package_cache::PackageCache,
    per_file_ignores::PerFileIgnores,
    rule_options::ResolvedRuleOptions,
    rule_set::{ALL_RULES, Category, Rule, RuleSet},
    settings::Settings,
};
use air_r_syntax::RSyntaxKind;
use anyhow::Result;
use std::{collections::HashSet, path::PathBuf, sync::Arc};

use crate::lints::base::assignment::options::ResolvedAssignmentOptions;

/// Parsed rule selection from CLI or TOML configuration.
///
/// `selected` / `extended` are `None` when the setting was not given at all,
/// which `reconcile_rules` treats differently from an empty selection.
#[derive(Debug)]
pub struct RuleSelection {
    pub selected: Option<HashSet<Rule>>,
    pub extended: Option<HashSet<Rule>>,
    pub ignored: HashSet<Rule>,
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
    /// Directories the user declared as the project being checked: a directory
    /// argument (`jarl check .`) or the directory holding the `jarl.toml` that
    /// governs it. Scanned in full so files can see each other even outside a
    /// package; see [`crate::db::AnalysisDb::build`]. Empty when only file
    /// arguments were given, which keeps `jarl check /tmp/foo.R` from walking
    /// `/tmp`.
    pub project_roots: Vec<PathBuf>,
    /// List of rules and whether they have an associated safe fix, passed by
    /// the user and/or recovered from the config file. Those will
    /// not necessarily all be used, for instance if we disable unsafe fixes.
    pub rules: RuleSet,
    /// List of rules to use. If we lint only, then this is equivalent to the
    /// field `rules`. If we apply fixes too, then this might be different from
    /// `rules` because it may filter out rules that have unsafe fixes.
    pub rules_to_apply: RuleSet,
    /// Whether safe fixes should be applied.
    pub apply_fixes: bool,
    /// Did the user pass the --unsafe-fixes flag?
    pub apply_unsafe_fixes: bool,
    /// The minimum R version the user pinned with `--min-r-version`, which
    /// applies to the whole run and overrides every package's own `Depends`.
    pub minimum_r_version: Option<(u32, u32, u32)>,
    /// Apply fixes even if the Git branch still has uncommitted files?
    pub allow_dirty: bool,
    /// Apply fixes even if there is no version control system?
    pub allow_no_vcs: bool,
    /// Rules that should not have their fixes applied (from unfixable setting)
    pub unfixable: HashSet<Rule>,
    /// Rules that are allowed to have fixes applied (from fixable setting)
    /// None means all rules with fixes can be applied
    pub fixable: Option<HashSet<Rule>>,
    /// Whether to lint R code inside roxygen `@examples` sections
    pub check_roxygen: bool,
    /// Whether to apply autofixes to roxygen examples
    pub fix_roxygen: bool,
    /// Resolved per-rule options (wrapped in Arc to avoid expensive clones)
    pub rule_options: Arc<ResolvedRuleOptions>,
    /// Shared cache of installed R package metadata for package-specific rules.
    /// `None` if library path discovery was not performed (e.g., no package rules enabled).
    pub package_cache: Option<Arc<PackageCache>>,
    /// Per-file rule ignores resolved from `[lint.per-file-ignores]`.
    pub per_file_ignores: PerFileIgnores,
}

pub fn build_config(
    check_config: &ArgsConfig,
    toml_settings: Option<&Settings>,
    paths: Vec<PathBuf>,
) -> Result<Config> {
    // The `--min-r-version` override, if the user passed one. A package's own
    // floor is resolved later, per file.
    let minimum_r_version = determine_minimum_r_version(check_config)?;

    let rules_cli = parse_rules_cli(
        &check_config.select,
        &check_config.extend_select,
        &check_config.ignore,
    )?;
    let rules_toml = parse_rules_toml(toml_settings)?;
    let rules = reconcile_rules(rules_cli, rules_toml)?;

    // We can only do this general filter if the user passed an explicity `--min-r-version`,
    // otherwise we resolve the min R version of each path later on. This is
    // necessary because not all paths necessarily get the same min R version.
    let rules = match minimum_r_version {
        Some(_) => filter_rules_by_version(&rules, minimum_r_version),
        None => rules,
    };

    // Parse fixable/unfixable rules from TOML.
    // These will be stored in Config and checked when applying fixes.
    let (fixable_toml, unfixable_toml) = parse_fixable_toml(toml_settings)?;

    // --fix-only implies --fix, while --unsafe-fixes controls which fixes can
    // be applied independently of how fix mode was enabled.
    let apply_fixes = check_config.fix || check_config.fix_only;
    let rules_to_apply = match (apply_fixes, check_config.unsafe_fixes) {
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
        rules_to_apply
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

    let check_roxygen = toml_settings
        .and_then(|s| s.linter.check_roxygen)
        .unwrap_or(true);

    let fix_roxygen = toml_settings
        .and_then(|s| s.linter.fix_roxygen)
        .unwrap_or(false);

    let per_file_ignores = toml_settings
        .map(|s| s.linter.per_file_ignores.clone())
        .unwrap_or_default();

    Ok(Config {
        project_roots: project_roots(&check_config.files),
        paths,
        rules,
        rules_to_apply,
        apply_fixes,
        apply_unsafe_fixes: check_config.unsafe_fixes,
        minimum_r_version,
        allow_dirty: check_config.allow_dirty,
        allow_no_vcs: check_config.allow_no_vcs,
        unfixable: unfixable_toml,
        fixable: fixable_toml,
        check_roxygen,
        fix_roxygen,
        rule_options: Arc::new(rule_options),
        package_cache: None,
        per_file_ignores,
    })
}

/// The project directories implied by the user's arguments.
///
/// A directory argument declares that directory as the project. A file
/// argument declares nothing on its own, but a `jarl.toml` / `.jarl.toml`
/// sitting next to it marks the project it belongs to, so that directory
/// counts too. Passing a bare file in a directory with no config therefore
/// contributes no root, which is what keeps `jarl check /tmp/foo.R` from
/// declaring `/tmp` a project.
///
/// Nested roots are collapsed to their outermost ancestor so a tree is only
/// ever declared once.
fn project_roots(args: &[PathBuf]) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = args
        .iter()
        .filter_map(|arg| {
            if arg.is_dir() {
                return Some(arg.clone());
            }
            // A file argument only pulls in its directory when that directory
            // is configured, i.e. the user has told us where the project is.
            let dir = arg.parent()?;
            crate::toml::find_jarl_toml_in_directory(dir)?;
            Some(dir.to_path_buf())
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    roots.sort();

    let mut outermost: Vec<PathBuf> = Vec::new();
    for root in roots {
        if !outermost.iter().any(|ancestor| root.starts_with(ancestor)) {
            outermost.push(root);
        }
    }
    outermost
}

/// Expand rule groups (`PERF`, `ALL`) and reject anything that isn't a rule.
///
/// This is the one validation path for every place a user can name rules:
/// `--select` / `--extend-select` / `--ignore`, the matching `jarl.toml`
/// fields, `fixable` / `unfixable`, and `[lint.per-file-ignores]`. `field`
/// names the setting in the error message, e.g. `` "`--select`" `` or
/// `` "field `select` in 'jarl.toml'" ``.
pub(crate) fn resolve_rule_names<'a>(
    names: impl IntoIterator<Item = &'a str>,
    field: &str,
) -> Result<Vec<Rule>> {
    let all_rules = Rule::all();
    let passed_by_user: Vec<&str> = names.into_iter().collect();
    let expanded_rules = replace_group_rules(&passed_by_user, all_rules);

    if let Some(invalid) = get_invalid_rules(all_rules, &expanded_rules) {
        return Err(unknown_rules_error(
            format!("Unknown rules in {field}: {}", invalid.names.join(", ")),
            invalid.help,
        ));
    }

    // `get_invalid_rules` just proved every name resolves, and
    // `replace_group_rules` already trimmed them.
    Ok(expanded_rules
        .iter()
        .filter_map(|name| Rule::from_name(name))
        .collect())
}

/// [`resolve_rule_names`] collected into a set, for the settings matched
/// against a `Diagnostic`'s rule (`fixable` / `unfixable`).
fn resolve_rule_name_set<'a>(
    names: impl IntoIterator<Item = &'a str>,
    field: &str,
) -> Result<HashSet<Rule>> {
    Ok(resolve_rule_names(names, field)?.into_iter().collect())
}

/// Parse CLI rule arguments into a [`RuleSelection`].
///
/// `selected` / `extended` are `None` when the corresponding flag was not
/// passed, which is different from being passed an empty selection.
pub fn parse_rules_cli(select: &str, extend_select: &str, ignore: &str) -> Result<RuleSelection> {
    let split = |value: &str| -> Option<Vec<String>> {
        (!value.is_empty()).then(|| value.split(',').map(str::to_string).collect())
    };

    Ok(RuleSelection {
        selected: resolve_optional(split(select).as_deref(), "`--select`")?,
        extended: resolve_optional(split(extend_select).as_deref(), "`--extend-select`")?,
        ignored: resolve_optional(split(ignore).as_deref(), "`--ignore`")?.unwrap_or_default(),
    })
}

/// Parse the rule selection from `jarl.toml`.
pub fn parse_rules_toml(toml_settings: Option<&Settings>) -> Result<RuleSelection> {
    let Some(settings) = toml_settings else {
        return Ok(RuleSelection {
            selected: None,
            extended: None,
            ignored: HashSet::new(),
        });
    };
    let linter = &settings.linter;

    Ok(RuleSelection {
        selected: resolve_optional(linter.select.as_deref(), "field `select` in 'jarl.toml'")?,
        extended: resolve_optional(
            linter.extend_select.as_deref(),
            "field `extend-select` in 'jarl.toml'",
        )?,
        ignored: resolve_optional(linter.ignore.as_deref(), "field `ignore` in 'jarl.toml'")?
            .unwrap_or_default(),
    })
}

/// Resolve an optional list of rule names, keeping "absent" distinct from
/// "empty".
fn resolve_optional(names: Option<&[String]>, field: &str) -> Result<Option<HashSet<Rule>>> {
    names
        .map(|names| {
            resolve_rule_names(names.iter().map(String::as_str), field)
                .map(|rules| rules.into_iter().collect())
        })
        .transpose()
}

/// Parse `fixable` / `unfixable` from `jarl.toml`.
///
/// `fixable` is `None` when unset, meaning every rule with a fix may apply it.
pub fn parse_fixable_toml(
    toml_settings: Option<&Settings>,
) -> Result<(Option<HashSet<Rule>>, HashSet<Rule>)> {
    let Some(settings) = toml_settings else {
        return Ok((None, HashSet::new()));
    };
    let linter = &settings.linter;

    let fixable = linter
        .fixable
        .as_ref()
        .map(|names| {
            resolve_rule_name_set(
                names.iter().map(String::as_str),
                "field `fixable` in 'jarl.toml'",
            )
        })
        .transpose()?;

    let unfixable = linter
        .unfixable
        .as_ref()
        .map(|names| {
            resolve_rule_name_set(
                names.iter().map(String::as_str),
                "field `unfixable` in 'jarl.toml'",
            )
        })
        .transpose()?
        .unwrap_or_default();

    Ok((fixable, unfixable))
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
/// Invalid rule names found in a configuration field, plus "did you mean"
/// help lines for the ones close to a real rule name.
struct InvalidRules {
    /// Invalid names as they should appear in the "Unknown rules: ..." message.
    names: Vec<String>,
    /// One help line per invalid name that has a suggestion, e.g.
    /// `Did you mean "glue"?`.
    help: Vec<String>,
}

fn get_invalid_rules(
    all_rule_names: &[Rule],
    rules_passed_by_user: &[String],
) -> Option<InvalidRules> {
    let all_rules_set: HashSet<_> = all_rule_names.iter().map(|x| x.name()).collect();

    // Candidates for "did you mean" suggestions: rule names, category group
    // names (e.g. "PERF"), and the special "ALL" keyword.
    let suggestion_candidates: Vec<&str> = all_rule_names
        .iter()
        .map(|x| x.name())
        .chain(Category::ALL.iter().map(|c| c.as_str()))
        .chain(std::iter::once("ALL"))
        .collect();

    let mut names = Vec::new();
    let mut help = Vec::new();

    for rule in rules_passed_by_user {
        let trimmed = rule.trim();

        // Rule is invalid if it's empty/whitespace-only or doesn't exist.
        if trimmed.is_empty() {
            names.push(format!("\"{rule}\" (empty or whitespace-only not allowed)"));
            continue;
        }
        if all_rules_set.contains(trimmed) {
            continue;
        }

        names.push(rule.clone());

        let suggestions = suggest_rule_names(trimmed, &suggestion_candidates);
        match suggestions.as_slice() {
            [] => {}
            [only] => help.push(format!("Did you mean \"{only}\"?")),
            many => {
                let quoted = many
                    .iter()
                    .map(|s| format!("\"{s}\""))
                    .collect::<Vec<_>>()
                    .join(", ");
                help.push(format!("Did you mean one of {quoted}?"));
            }
        }
    }

    if names.is_empty() {
        None
    } else {
        Some(InvalidRules { names, help })
    }
}

/// Build an `Unknown rules` error carrying optional "did you mean" help lines.
fn unknown_rules_error(message: String, help: Vec<String>) -> anyhow::Error {
    anyhow::Error::new(UnknownRulesError { message, help })
}

/// Find valid rule names closest to `input` for a "did you mean" suggestion.
///
/// Returns up to 3 candidates that all share the minimum edit distance, and
/// only when that distance is within an acceptance threshold (so unrelated
/// input like "foo" yields no suggestion). Uses Damerau-Levenshtein distance,
/// which also accounts for adjacent transpositions (e.g. "treu" -> "true").
/// Suggest known rule names close to `input`, for "did you mean" hints when a
/// user passes an unknown rule (e.g. to `jarl rule <name>`).
pub fn suggest_rules(input: &str) -> Vec<String> {
    let candidates: Vec<&str> = crate::rule_set::ALL_RULES
        .iter()
        .map(|rule| rule.name())
        .collect();
    suggest_rule_names(input, &candidates)
}

fn suggest_rule_names(input: &str, candidates: &[&str]) -> Vec<String> {
    // Allow roughly one edit per three characters, with a floor of 1 so short
    // typos are still caught.
    let threshold = (input.chars().count() / 3).max(1);

    let mut scored: Vec<(usize, &str)> = candidates
        .iter()
        .map(|&name| (strsim::damerau_levenshtein(input, name), name))
        .filter(|(distance, _)| *distance <= threshold)
        .collect();

    let Some(&(best_distance, _)) = scored.iter().min_by_key(|(distance, _)| *distance) else {
        return Vec::new();
    };

    scored.retain(|(distance, _)| *distance == best_distance);
    scored.sort_by(|a, b| a.1.cmp(b.1));
    scored
        .into_iter()
        .take(3)
        .map(|(_, name)| name.to_string())
        .collect()
}

/// Reconcile rules from CLI and TOML configuration.
///
/// Strategy:
/// - CLI select takes precedence over TOML select
/// - CLI ignore and TOML ignore are combined (both applied)
/// - If neither CLI nor TOML specify select, start with all rules
fn reconcile_rules(rules_cli: RuleSelection, rules_toml: RuleSelection) -> Result<RuleSet> {
    // CLI select wins over TOML select; with neither, start from the defaults.
    let mut selected = rules_cli
        .selected
        .or(rules_toml.selected)
        .unwrap_or_else(|| Rule::enabled_by_default().collect());

    // Same precedence for extend-select, but it adds to the base instead of
    // replacing it.
    if let Some(extended) = rules_cli.extended.or(rules_toml.extended) {
        selected.extend(extended);
    }

    // Both ignore lists apply.
    for rule in rules_cli.ignored.iter().chain(rules_toml.ignored.iter()) {
        selected.remove(rule);
    }

    // Emit in declaration order rather than the set's, so anything downstream
    // that reports rules in order (e.g. package-specific categories) stays
    // deterministic.
    Ok(ALL_RULES
        .iter()
        .filter(|rule| selected.contains(rule))
        .collect())
}

/// The R version the user pinned with `--min-r-version`, if any.
///
/// We don't use a `Depends` field here. This is done later on, on a per-file
/// basis and if the user didn't pass `--min-r-version` because one call could
/// encompass several packages with different `Depends`.
fn determine_minimum_r_version(check_config: &ArgsConfig) -> Result<Option<(u32, u32, u32)>> {
    check_config
        .min_r_version
        .as_ref()
        .map(|version| parse_r_version(version.clone()))
        .transpose()
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
pub(crate) fn filter_rules_by_version(
    rules: &RuleSet,
    minimum_r_version: Option<(u32, u32, u32)>,
) -> RuleSet {
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
