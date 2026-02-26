const DEFAULT_THRESHOLD_IGNORE: usize = 50;

/// TOML options for `[lint.unused-function]`.
///
/// Use `threshold-ignore` to control when `unused_internal_function`
/// diagnostics are hidden. When the number of violations exceeds this
/// threshold, they are suppressed with an informative note (likely false
/// positives).
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct UnusedFunctionOptions {
    pub threshold_ignore: Option<usize>,
}

/// Resolved options for the `unused_internal_function` rule.
#[derive(Clone, Debug)]
pub struct ResolvedUnusedFunctionOptions {
    pub threshold_ignore: usize,
}

impl ResolvedUnusedFunctionOptions {
    pub fn resolve(options: Option<&UnusedFunctionOptions>) -> anyhow::Result<Self> {
        let threshold_ignore = options
            .and_then(|opts| opts.threshold_ignore)
            .unwrap_or(DEFAULT_THRESHOLD_IGNORE);

        Ok(Self { threshold_ignore })
    }
}
