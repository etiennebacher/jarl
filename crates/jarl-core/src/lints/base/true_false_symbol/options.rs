use std::collections::HashSet;

/// TOML options for `[lint.true_false_symbol]`.
///
/// Use `skipped-functions` to list functions whose arguments are allowed to
/// contain the `T` and `F` symbols. This list is empty by default.
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct TrueFalseSymbolOptions {
    pub skipped_functions: Option<Vec<String>>,
}

/// Resolved options for the `true_false_symbol` rule, ready for use during
/// linting.
#[derive(Clone, Debug)]
pub struct ResolvedTrueFalseSymbolOptions {
    pub skipped_functions: HashSet<String>,
}

impl ResolvedTrueFalseSymbolOptions {
    pub fn resolve(options: Option<&TrueFalseSymbolOptions>) -> anyhow::Result<Self> {
        let skipped_functions = options
            .and_then(|opts| opts.skipped_functions.as_ref())
            .map(|values| values.iter().cloned().collect())
            .unwrap_or_default();

        Ok(Self { skipped_functions })
    }
}
