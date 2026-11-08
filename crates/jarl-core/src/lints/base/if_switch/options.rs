const DEFAULT_MAX_BRANCHES: usize = 3;

/// <!-- docs: start -->
/// Use `max-branches` to set the number of equality branches (`if` and
/// `else if` conditions) from which an `if`/`else` chain is reported. It must
/// be at least 2.
///
/// ### Default values
///
/// ```toml
/// max-branches = 3
/// ```
///
/// ### TOML settings
///
/// ```toml
/// [lint]
/// ...
///
/// [lint.if_switch]
/// max-branches = 4
/// ```
/// <!-- docs: end -->
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct IfSwitchOptions {
    pub max_branches: Option<usize>,
}

/// Resolved options for the `if_switch` rule, ready for use during linting.
#[derive(Clone, Debug)]
pub struct ResolvedIfSwitchOptions {
    pub max_branches: usize,
}

impl ResolvedIfSwitchOptions {
    pub fn resolve(options: Option<&IfSwitchOptions>) -> anyhow::Result<Self> {
        let max_branches = options
            .and_then(|opts| opts.max_branches)
            .unwrap_or(DEFAULT_MAX_BRANCHES);

        // A single `if` is not a chain that `switch()` could replace.
        if max_branches < 2 {
            return Err(anyhow::anyhow!(
                "`max-branches` in `[lint.if_switch]` must be at least 2."
            ));
        }

        Ok(Self { max_branches })
    }
}
