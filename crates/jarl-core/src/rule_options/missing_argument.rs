use std::collections::HashSet;

use super::resolve_with_extend;

/// Default functions whose empty arguments are not reported.
const DEFAULT_SKIPPED_FUNCTIONS: &[&str] = &[
    "switch",
    "tibble",
    "list2",
    "mutate",
    "summarize",
    "transmute",
];

/// TOML options for `[lint.missing_argument]`.
///
/// Use `skipped-functions` to fully replace the default list of functions
/// whose empty arguments are allowed. Use `extend-skipped-functions` to add
/// to the default list. Specifying both is an error.
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct MissingArgumentOptions {
    pub skipped_functions: Option<Vec<String>>,
    pub extend_skipped_functions: Option<Vec<String>>,
}

/// Resolved options for the `missing_argument` rule, ready for use during
/// linting.
#[derive(Clone, Debug)]
pub struct ResolvedMissingArgumentOptions {
    pub skipped_functions: HashSet<String>,
}

impl ResolvedMissingArgumentOptions {
    pub fn resolve(options: Option<&MissingArgumentOptions>) -> anyhow::Result<Self> {
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
            "missing_argument",
            "skipped-functions",
        )?;

        Ok(Self { skipped_functions })
    }
}
