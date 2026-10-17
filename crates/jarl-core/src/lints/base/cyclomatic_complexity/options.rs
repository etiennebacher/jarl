const DEFAULT_MAX_COMPLEXITY: usize = 20;

/// <!-- docs: start -->
/// Use `max-complexity` to set the highest score a function is allowed to
/// reach before it is reported. It must be at least 1.
///
/// ### Default values
///
/// ```toml
/// max-complexity = 20
/// ```
///
/// ### TOML settings
///
/// ```toml
/// [lint]
/// ...
///
/// [lint.cyclomatic_complexity]
/// max-complexity = 25
/// ```
/// <!-- docs: end -->
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct CyclomaticComplexityOptions {
    pub max_complexity: Option<usize>,
}

/// Resolved options for the `cyclomatic_complexity` rule, ready for use during
/// linting.
#[derive(Clone, Debug)]
pub struct ResolvedCyclomaticComplexityOptions {
    pub max_complexity: usize,
}

impl ResolvedCyclomaticComplexityOptions {
    pub fn resolve(options: Option<&CyclomaticComplexityOptions>) -> anyhow::Result<Self> {
        let max_complexity = options
            .and_then(|opts| opts.max_complexity)
            .unwrap_or(DEFAULT_MAX_COMPLEXITY);

        // The score of a function is never below 1, so a maximum of 0 would
        // report every single expression.
        if max_complexity == 0 {
            return Err(anyhow::anyhow!(
                "`max-complexity` in `[lint.cyclomatic_complexity]` must be at least 1."
            ));
        }

        Ok(Self { max_complexity })
    }
}
