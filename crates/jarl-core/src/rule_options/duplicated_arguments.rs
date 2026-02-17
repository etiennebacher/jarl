use std::collections::HashSet;

use super::resolve_with_extend;

/// Default functions that are allowed to have duplicated arguments.
const DEFAULT_SKIPPED_FUNCTIONS: &[&str] = &["c", "mutate", "summarize", "transmute"];

/// TOML options for `[lint.duplicated-arguments]`.
///
/// Use `skipped-functions` to fully replace the default list of functions
/// that are allowed to have duplicated arguments. Use
/// `extend-skipped-functions` to add to the default list.
/// Specifying both is an error.
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct DuplicatedArgumentsOptions {
    pub skipped_functions: Option<Vec<String>>,
    pub extend_skipped_functions: Option<Vec<String>>,
}

/// Resolved options for the `duplicated_arguments` rule, ready for use during
/// linting.
#[derive(Clone, Debug)]
pub struct ResolvedDuplicatedArgumentsOptions {
    pub skipped_functions: HashSet<String>,
}

impl ResolvedDuplicatedArgumentsOptions {
    pub fn resolve(options: Option<&DuplicatedArgumentsOptions>) -> anyhow::Result<Self> {
        let (base, extend) = match options {
            Some(opts) => (
                opts.skipped_functions.as_ref(),
                opts.extend_skipped_functions.as_ref(),
            ),
            None => (None, None),
        };

        let skipped_functions = resolve_with_extend(
            base,
            extend,
            DEFAULT_SKIPPED_FUNCTIONS,
            "duplicated-arguments",
            "skipped-functions",
        )?;

        Ok(Self { skipped_functions })
    }
}
