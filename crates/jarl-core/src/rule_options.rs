use std::collections::HashSet;

use crate::lints::base::assignment::options::AssignmentOptions;
use crate::lints::base::assignment::options::ResolvedAssignmentOptions;
use crate::lints::base::duplicated_arguments::options::DuplicatedArgumentsOptions;
use crate::lints::base::duplicated_arguments::options::ResolvedDuplicatedArgumentsOptions;
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

/// Resolved per-rule options, ready for use during linting.
///
/// To add options for a new rule:
/// 1. Create `lints/<group>/<rule_name>/options.rs` with the TOML and resolved
///    types, and declare `pub(crate) mod options;` in the rule's `mod.rs`.
/// 2. Add a field to `ResolvedRuleOptions` and a resolve line in `resolve()`.
/// 3. Add the TOML field to `LinterTomlOptions` in `toml.rs` and pass it to
///    `resolve()` in `into_settings()`.
#[derive(Clone, Debug)]
pub struct ResolvedRuleOptions {
    pub assignment: ResolvedAssignmentOptions,
    pub duplicated_arguments: ResolvedDuplicatedArgumentsOptions,
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
    #[allow(clippy::too_many_arguments)]
    pub fn resolve(
        assignment: Option<&AssignmentOptions>,
        duplicated_arguments: Option<&DuplicatedArgumentsOptions>,
        implicit_assignment: Option<&ImplicitAssignmentOptions>,
        missing_argument: Option<&MissingArgumentOptions>,
        nested_pipe: Option<&NestedPipeOptions>,
        pipe_consistency: Option<&PipeConsistencyOptions>,
        quotes: Option<&QuotesOptions>,
        true_false_symbol: Option<&TrueFalseSymbolOptions>,
        undesirable_function: Option<&UndesirableFunctionOptions>,
        unreachable_code: Option<&UnreachableCodeOptions>,
        unused_function: Option<&UnusedFunctionOptions>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            assignment: ResolvedAssignmentOptions::resolve(assignment)?,
            duplicated_arguments: ResolvedDuplicatedArgumentsOptions::resolve(
                duplicated_arguments,
            )?,
            implicit_assignment: ResolvedImplicitAssignmentOptions::resolve(implicit_assignment)?,
            missing_argument: ResolvedMissingArgumentOptions::resolve(missing_argument)?,
            nested_pipe: ResolvedNestedPipeOptions::resolve(nested_pipe)?,
            pipe_consistency: ResolvedPipeConsistencyOptions::resolve(pipe_consistency)?,
            quotes: ResolvedQuotesOptions::resolve(quotes)?,
            true_false_symbol: ResolvedTrueFalseSymbolOptions::resolve(true_false_symbol)?,
            undesirable_function: ResolvedUndesirableFunctionOptions::resolve(
                undesirable_function,
            )?,
            unreachable_code: ResolvedUnreachableCodeOptions::resolve(unreachable_code)?,
            unused_function: ResolvedUnusedFunctionOptions::resolve(unused_function)?,
        })
    }
}

impl Default for ResolvedRuleOptions {
    fn default() -> Self {
        Self::resolve(
            None, None, None, None, None, None, None, None, None, None, None,
        )
        .expect("default rule options should always resolve")
    }
}
