use std::collections::HashSet;

use super::resolve_with_extend;

/// Default outer calls whose nested pipes are allowed.
const DEFAULT_SKIPPED_FUNCTIONS: &[&str] = &["try", "tryCatch", "withCallingHandlers"];

/// TOML options for `[lint.nested_pipe]`.
///
/// Use `skipped-functions` to fully replace the default list of outer calls
/// whose nested pipes are allowed. Use `extend-skipped-functions` to add to the
/// default list. Specifying both is an error.
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct NestedPipeOptions {
    pub skipped_functions: Option<Vec<String>>,
    pub extend_skipped_functions: Option<Vec<String>>,
}

/// Resolved options for the `nested_pipe` rule, ready for use during linting.
#[derive(Clone, Debug)]
pub struct ResolvedNestedPipeOptions {
    pub skipped_functions: HashSet<String>,
}

impl ResolvedNestedPipeOptions {
    pub fn resolve(options: Option<&NestedPipeOptions>) -> anyhow::Result<Self> {
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
            "nested_pipe",
            "skipped-functions",
        )?;

        Ok(Self { skipped_functions })
    }
}
