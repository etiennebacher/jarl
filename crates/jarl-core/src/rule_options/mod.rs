pub mod assignment;
pub mod duplicated_arguments;
pub mod unreachable_code;

use assignment::AssignmentOptions;
use assignment::ResolvedAssignmentOptions;
use duplicated_arguments::DuplicatedArgumentsOptions;
use duplicated_arguments::ResolvedDuplicatedArgumentsOptions;
use std::collections::HashSet;
use unreachable_code::ResolvedUnreachableCodeOptions;
use unreachable_code::UnreachableCodeOptions;

/// Resolve a pair of `field` / `extend-field` options against a set of defaults.
///
/// - If both are `Some`, returns an error.
/// - If `base` is `Some`, uses it as the full replacement.
/// - If `extend` is `Some`, merges it with the defaults.
/// - If neither is set, returns the defaults.
///
/// `rule_section` and `field_name` are used for the error message, e.g.
/// `"duplicated-arguments"` and `"skipped-functions"`.
pub fn resolve_with_extend(
    base: Option<&Vec<String>>,
    extend: Option<&Vec<String>>,
    defaults: &[&str],
    rule_section: &str,
    field_name: &str,
) -> anyhow::Result<HashSet<String>> {
    if base.is_some() && extend.is_some() {
        return Err(anyhow::anyhow!(
            "Cannot specify both `{field_name}` and `extend-{field_name}` \
             in `[lint.{rule_section}]`."
        ));
    }

    let default_set: HashSet<String> = defaults.iter().map(|s| (*s).to_string()).collect();

    if let Some(values) = base {
        Ok(values.iter().cloned().collect())
    } else if let Some(values) = extend {
        let mut set = default_set;
        set.extend(values.iter().cloned());
        Ok(set)
    } else {
        Ok(default_set)
    }
}

/// Resolved per-rule options, ready for use during linting.
///
/// To add options for a new rule:
/// 1. Create `rule_options/<rule_name>.rs` with the TOML and resolved types.
/// 2. Add a field to `ResolvedRuleOptions` and a resolve line in `resolve()`.
/// 3. Add the TOML field to `LinterTomlOptions` in `toml.rs` and pass it to
///    `resolve()` in `into_settings()`.
#[derive(Clone, Debug)]
pub struct ResolvedRuleOptions {
    pub assignment: ResolvedAssignmentOptions,
    pub duplicated_arguments: ResolvedDuplicatedArgumentsOptions,
    pub unreachable_code: ResolvedUnreachableCodeOptions,
}

impl ResolvedRuleOptions {
    pub fn resolve(
        assignment: Option<&AssignmentOptions>,
        duplicated_arguments: Option<&DuplicatedArgumentsOptions>,
        unreachable_code: Option<&UnreachableCodeOptions>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            assignment: ResolvedAssignmentOptions::resolve(assignment)?,
            duplicated_arguments: ResolvedDuplicatedArgumentsOptions::resolve(
                duplicated_arguments,
            )?,
            unreachable_code: ResolvedUnreachableCodeOptions::resolve(unreachable_code)?,
        })
    }
}

impl Default for ResolvedRuleOptions {
    fn default() -> Self {
        Self::resolve(None, None, None).expect("default rule options should always resolve")
    }
}
