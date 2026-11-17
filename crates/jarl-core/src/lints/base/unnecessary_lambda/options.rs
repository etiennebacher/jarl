/// <!-- docs: start -->
/// Use `allow-comparison = true` to not report anonymous functions that
/// compare the output of a function to a constant in `sapply()` or `vapply()`,
/// e.g. `sapply(x, function(xi) foo(xi) == 2)`.
///
/// ### Default values
///
/// ```toml
/// allow-comparison = false
/// ```
///
/// ### TOML settings
///
/// ```toml
/// [lint]
/// ...
///
/// [lint.unnecessary_lambda]
/// allow-comparison = true
/// ```
/// <!-- docs: end -->
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct UnnecessaryLambdaOptions {
    pub allow_comparison: Option<bool>,
}

/// Resolved options for the `unnecessary_lambda` rule, ready for use during
/// linting.
#[derive(Clone, Debug)]
pub struct ResolvedUnnecessaryLambdaOptions {
    pub allow_comparison: bool,
}

impl ResolvedUnnecessaryLambdaOptions {
    pub fn resolve(options: Option<&UnnecessaryLambdaOptions>) -> anyhow::Result<Self> {
        let allow_comparison = options
            .and_then(|opts| opts.allow_comparison)
            .unwrap_or(false);

        Ok(Self { allow_comparison })
    }
}
