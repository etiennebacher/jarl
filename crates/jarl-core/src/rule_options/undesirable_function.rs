use std::collections::HashSet;

use super::resolve_with_extend;

/// Default functions that are considered undesirable.
const DEFAULT_FUNCTIONS: &[&str] = &["browser"];

/// TOML options for `[lint.undesirable-function]`.
///
/// Use `functions` to fully replace the default list of undesirable functions.
/// Use `extend-functions` to add to the default list.
/// Specifying both is an error.
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct UndesirableFunctionOptions {
    pub functions: Option<Vec<String>>,
    pub extend_functions: Option<Vec<String>>,
}

/// Resolved options for the `undesirable_function` rule, ready for use during
/// linting.
#[derive(Clone, Debug)]
pub struct ResolvedUndesirableFunctionOptions {
    pub functions: HashSet<String>,
}

impl ResolvedUndesirableFunctionOptions {
    pub fn resolve(options: Option<&UndesirableFunctionOptions>) -> anyhow::Result<Self> {
        let (base, extend) = match options {
            Some(opts) => (opts.functions.as_ref(), opts.extend_functions.as_ref()),
            None => (None, None),
        };

        let functions = resolve_with_extend(
            base,
            extend,
            DEFAULT_FUNCTIONS,
            "undesirable-function",
            "functions",
        )?;

        Ok(Self { functions })
    }
}
