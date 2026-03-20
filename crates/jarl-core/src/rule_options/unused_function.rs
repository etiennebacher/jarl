use regex::Regex;

const DEFAULT_THRESHOLD_IGNORE: usize = 50;

/// TOML options for `[lint.unused_function]`.
///
/// Use `threshold-ignore` to control when `unused_function`
/// diagnostics are hidden. When the number of violations exceeds this
/// threshold, they are suppressed with an informative note (likely false
/// positives).
///
/// Use `skipped-functions` to provide a list of regex patterns for
/// functions that should be skipped by this rule.
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct UnusedFunctionOptions {
    pub threshold_ignore: Option<usize>,
    pub skipped_functions: Option<Vec<String>>,
}

/// Resolved options for the `unused_function` rule.
#[derive(Clone, Debug)]
pub struct ResolvedUnusedFunctionOptions {
    pub threshold_ignore: usize,
    pub skipped_functions: Vec<Regex>,
}

impl ResolvedUnusedFunctionOptions {
    pub fn resolve(options: Option<&UnusedFunctionOptions>) -> anyhow::Result<Self> {
        let threshold_ignore = options
            .and_then(|opts| opts.threshold_ignore)
            .unwrap_or(DEFAULT_THRESHOLD_IGNORE);

        let skipped_functions = match options.and_then(|opts| opts.skipped_functions.as_ref()) {
            Some(patterns) => patterns
                .iter()
                .map(|p| {
                    Regex::new(p).map_err(|e| {
                        anyhow::anyhow!(
                            "Invalid regex `{p}` in `skipped-functions` \
                             of `[lint.unused_function]`: {e}"
                        )
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
            None => Vec::new(),
        };

        Ok(Self { threshold_ignore, skipped_functions })
    }

    /// Returns `true` if the given function name matches any of the
    /// `skipped-functions` patterns.
    pub fn is_skipped(&self, name: &str) -> bool {
        self.skipped_functions.iter().any(|re| re.is_match(name))
    }
}
