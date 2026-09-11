use regex::Regex;

const DEFAULT_THRESHOLD_IGNORE: usize = 50;

/// <!-- docs: start -->
/// Use `skipped-functions` to fully replace the default list of functions that are
/// allowed to be unused in the R package. Function names in `skipped-functions`
/// **are parsed as regular expressions** (this differs from other rules that have a
/// `skipped-functions` argument).
/// 
/// `unused_function` might return false positives because Jarl cannot statically
/// determine whether a function is used. By default, Jarl will hide `unused_function`
/// diagnostics if there are more than 50, as this would suggest that the package
/// has some internal mechanism to use those functions. This number can be changed
/// with the `threshold-ignore` argument.
/// 
/// Defaults:
/// 
/// - `skipped-functions = []`
/// - `threshold-ignore = 50`
/// 
/// ```toml
/// [lint]
/// ...
/// 
/// [lint.unused_function]
/// # Ignore all functions that start with "pl_" or "cs_", and the function
/// # "my.function"
/// skipped-functions = ["^cs_", "^pl_", "my\\.function"]
/// # Set a custom threshold above which diagnostics for this rule aren't reported
/// # (this is basically equivalent to never hiding unused functions).
/// threshold-ignore = 10000
/// ```
/// <!-- docs: end -->
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
