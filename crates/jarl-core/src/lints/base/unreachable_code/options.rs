use std::collections::HashSet;

use crate::rule_options::resolve_with_extend;

/// Default functions that stop execution (never return).
const DEFAULT_STOPPING_FUNCTIONS: &[&str] =
    &["stop", ".Defunct", "abort", "cli_abort", "q", "quit"];

/// <!-- docs: start -->
/// Use `stopping-functions` to fully replace the default list of functions that are
/// considered to stop execution (never return). Use `extend-stopping-functions` to
/// add to the default list. Specifying both is an error.
/// 
/// Function names in `stopping-functions` or `extend-stopping-functions` also match
/// namespaced calls, e.g. `stopping-functions = ["abort"]` will consider `abort()`
/// and `rlang::abort()` as stopping functions.
/// 
/// Default: `stopping-functions = ["stop", ".Defunct", "abort", "cli_abort",
/// "q", "quit"]`.
/// 
/// ```toml
/// [lint]
/// ...
/// 
/// [lint.unreachable_code]
/// # Add a custom function to the list of stopping functions
/// extend-stopping-functions = ["my_custom_stop"]
/// ```
/// <!-- docs: end -->
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct UnreachableCodeOptions {
    pub stopping_functions: Option<Vec<String>>,
    pub extend_stopping_functions: Option<Vec<String>>,
}

/// Resolved options for the `unreachable_code` rule, ready for use during
/// linting.
#[derive(Clone, Debug)]
pub struct ResolvedUnreachableCodeOptions {
    pub stopping_functions: HashSet<String>,
}

impl ResolvedUnreachableCodeOptions {
    pub fn resolve(options: Option<&UnreachableCodeOptions>) -> anyhow::Result<Self> {
        let (base, extend) = match options {
            Some(opts) => (
                opts.stopping_functions.as_ref(),
                opts.extend_stopping_functions.as_ref(),
            ),
            None => (None, None),
        };

        let stopping_functions = resolve_with_extend(
            base,
            extend,
            DEFAULT_STOPPING_FUNCTIONS,
            "unreachable_code",
            "stopping-functions",
        )?;

        Ok(Self { stopping_functions })
    }
}
