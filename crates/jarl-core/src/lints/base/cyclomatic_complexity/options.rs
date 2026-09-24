use std::collections::HashSet;

const DEFAULT_MAX_COMPLEXITY: usize = 20;

/// Calls that leave the function without returning a value. Kept in sync with
/// the defaults of `unreachable_code`, but not configurable here: the score is
/// dominated by the branching constructs, so the exact list matters little.
const STOPPING_FUNCTIONS: &[&str] = &["stop", ".Defunct", "abort", "cli_abort", "q", "quit"];

/// <!-- docs: start -->
/// Use `max-complexity` to set the highest score a function (or the top-level code
/// of a file) is allowed to reach before it is reported. It must be at least 1.
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
/// # Only report the functions that are really tangled.
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
    pub stopping_functions: HashSet<String>,
}

impl ResolvedCyclomaticComplexityOptions {
    pub fn resolve(options: Option<&CyclomaticComplexityOptions>) -> anyhow::Result<Self> {
        let max_complexity = options
            .and_then(|opts| opts.max_complexity)
            .unwrap_or(DEFAULT_MAX_COMPLEXITY);

        // The score of a function is never below 1, so a maximum of 0 would
        // report every single function.
        if max_complexity == 0 {
            return Err(anyhow::anyhow!(
                "`max-complexity` in `[lint.cyclomatic_complexity]` must be at least 1."
            ));
        }

        Ok(Self {
            max_complexity,
            stopping_functions: STOPPING_FUNCTIONS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
        })
    }
}
