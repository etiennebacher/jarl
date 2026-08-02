use std::collections::HashSet;

use crate::lints::base::assignment::options::AssignmentOptions;
use crate::lints::base::assignment::options::ResolvedAssignmentOptions;
use crate::lints::base::duplicated_arguments::options::DuplicatedArgumentsOptions;
use crate::lints::base::duplicated_arguments::options::ResolvedDuplicatedArgumentsOptions;
use crate::lints::base::if_not_else::options::IfNotElseOptions;
use crate::lints::base::if_not_else::options::ResolvedIfNotElseOptions;
use crate::lints::base::implicit_assignment::options::ImplicitAssignmentOptions;
use crate::lints::base::implicit_assignment::options::ResolvedImplicitAssignmentOptions;
use crate::lints::base::missing_argument::options::MissingArgumentOptions;
use crate::lints::base::missing_argument::options::ResolvedMissingArgumentOptions;
use crate::lints::base::nested_pipe::options::NestedPipeOptions;
use crate::lints::base::nested_pipe::options::ResolvedNestedPipeOptions;
use crate::lints::base::pipe_consistency::options::PipeConsistencyOptions;
use crate::lints::base::pipe_consistency::options::ResolvedPipeConsistencyOptions;
use crate::lints::base::quotes::options::QuotesOptions;
use crate::lints::base::quotes::options::ResolvedQuotesOptions;
use crate::lints::base::true_false_symbol::options::ResolvedTrueFalseSymbolOptions;
use crate::lints::base::true_false_symbol::options::TrueFalseSymbolOptions;
use crate::lints::base::undesirable_function::options::ResolvedUndesirableFunctionOptions;
use crate::lints::base::undesirable_function::options::UndesirableFunctionOptions;
use crate::lints::base::unreachable_code::options::ResolvedUnreachableCodeOptions;
use crate::lints::base::unreachable_code::options::UnreachableCodeOptions;
use crate::lints::base::unused_function::options::ResolvedUnusedFunctionOptions;
use crate::lints::base::unused_function::options::UnusedFunctionOptions;

/// Resolve a pair of `field` / `extend-field` options against a set of defaults.
///
/// - If both are `Some`, returns an error.
/// - If `base` is `Some`, uses it as the full replacement.
/// - If `extend` is `Some`, merges it with the defaults.
/// - If neither is set, returns the defaults.
///
/// `rule_section` and `field_name` are used for the error message, e.g.
/// `"duplicated_arguments"` and `"skipped-functions"`.
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

/// Borrowed per-rule TOML options, grouped so they can be resolved in one go.
///
/// Fields default to `None` (no `[lint.<rule>]` table in the TOML), so call
/// sites only need to name the options they actually have.
#[derive(Debug, Default)]
pub struct RuleOptions<'a> {
    pub assignment: Option<&'a AssignmentOptions>,
    pub duplicated_arguments: Option<&'a DuplicatedArgumentsOptions>,
    pub if_not_else: Option<&'a IfNotElseOptions>,
    pub implicit_assignment: Option<&'a ImplicitAssignmentOptions>,
    pub missing_argument: Option<&'a MissingArgumentOptions>,
    pub nested_pipe: Option<&'a NestedPipeOptions>,
    pub pipe_consistency: Option<&'a PipeConsistencyOptions>,
    pub quotes: Option<&'a QuotesOptions>,
    pub true_false_symbol: Option<&'a TrueFalseSymbolOptions>,
    pub undesirable_function: Option<&'a UndesirableFunctionOptions>,
    pub unreachable_code: Option<&'a UnreachableCodeOptions>,
    pub unused_function: Option<&'a UnusedFunctionOptions>,
}

/// Resolved per-rule options, ready for use during linting.
///
/// To add options for a new rule:
/// 1. Create `lints/<group>/<rule_name>/options.rs` with the TOML and resolved
///    types, and declare `pub(crate) mod options;` in the rule's `mod.rs`.
/// 2. Add a field to `RuleOptions` and `ResolvedRuleOptions`, and a resolve
///    line in `resolve()`.
/// 3. Add the TOML field to `LinterTomlOptions` in `toml.rs` and set it on the
///    `RuleOptions` built in `into_settings()`.
#[derive(Clone, Debug)]
pub struct ResolvedRuleOptions {
    pub assignment: ResolvedAssignmentOptions,
    pub duplicated_arguments: ResolvedDuplicatedArgumentsOptions,
    pub if_not_else: ResolvedIfNotElseOptions,
    pub implicit_assignment: ResolvedImplicitAssignmentOptions,
    pub missing_argument: ResolvedMissingArgumentOptions,
    pub nested_pipe: ResolvedNestedPipeOptions,
    pub pipe_consistency: ResolvedPipeConsistencyOptions,
    pub quotes: ResolvedQuotesOptions,
    pub true_false_symbol: ResolvedTrueFalseSymbolOptions,
    pub undesirable_function: ResolvedUndesirableFunctionOptions,
    pub unreachable_code: ResolvedUnreachableCodeOptions,
    pub unused_function: ResolvedUnusedFunctionOptions,
}

impl ResolvedRuleOptions {
    pub fn resolve(options: &RuleOptions) -> anyhow::Result<Self> {
        Ok(Self {
            assignment: ResolvedAssignmentOptions::resolve(options.assignment)?,
            duplicated_arguments: ResolvedDuplicatedArgumentsOptions::resolve(
                options.duplicated_arguments,
            )?,
            if_not_else: ResolvedIfNotElseOptions::resolve(options.if_not_else)?,
            implicit_assignment: ResolvedImplicitAssignmentOptions::resolve(
                options.implicit_assignment,
            )?,
            missing_argument: ResolvedMissingArgumentOptions::resolve(options.missing_argument)?,
            nested_pipe: ResolvedNestedPipeOptions::resolve(options.nested_pipe)?,
            pipe_consistency: ResolvedPipeConsistencyOptions::resolve(options.pipe_consistency)?,
            quotes: ResolvedQuotesOptions::resolve(options.quotes)?,
            true_false_symbol: ResolvedTrueFalseSymbolOptions::resolve(options.true_false_symbol)?,
            undesirable_function: ResolvedUndesirableFunctionOptions::resolve(
                options.undesirable_function,
            )?,
            unreachable_code: ResolvedUnreachableCodeOptions::resolve(options.unreachable_code)?,
            unused_function: ResolvedUnusedFunctionOptions::resolve(options.unused_function)?,
        })
    }
}

impl Default for ResolvedRuleOptions {
    fn default() -> Self {
        Self::resolve(&RuleOptions::default()).expect("default rule options should always resolve")
    }
}
