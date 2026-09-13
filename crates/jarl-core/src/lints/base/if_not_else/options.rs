use std::collections::HashSet;

use crate::rule_options::resolve_with_extend;

/// Functions whose negated calls are allowed by default, e.g. `!is.null(x)`.
const DEFAULT_SKIPPED_FUNCTIONS: &[&str] = &["is.null", "is.na", "missing"];

/// <!-- docs: start -->
/// Use `skipped-functions` to fully replace the default list of functions whose
/// negated calls are allowed as an `if`/`ifelse()` condition (e.g. `!is.null(x)`).
/// Use `extend-skipped-functions` to add to the default list. Specifying both is an
/// error.
/// 
/// Function names in `skipped-functions` or `extend-skipped-functions` also match
/// namespaced calls, e.g. `skipped-functions = ["is.null"]` will allow `is.null()`
/// and `base::is.null()`.
/// 
/// Default: `skipped-functions = ["is.null", "is.na", "missing"]`
/// 
/// ```toml
/// [lint]
/// ...
/// 
/// [lint.if_not_else]
/// # Also allow a negated `is.data.frame()` call in the condition.
/// extend-skipped-functions = ["is.data.frame"]
/// ``` 
/// <!-- docs: end -->
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct IfNotElseOptions {
    pub skipped_functions: Option<Vec<String>>,
    pub extend_skipped_functions: Option<Vec<String>>,
}

/// Resolved options for the `if_not_else` rule, ready for use during linting.
#[derive(Clone, Debug)]
pub struct ResolvedIfNotElseOptions {
    pub skipped_functions: HashSet<String>,
}

impl ResolvedIfNotElseOptions {
    pub fn resolve(options: Option<&IfNotElseOptions>) -> anyhow::Result<Self> {
        let skipped_functions = resolve_with_extend(
            options.and_then(|opts| opts.skipped_functions.as_ref()),
            options.and_then(|opts| opts.extend_skipped_functions.as_ref()),
            DEFAULT_SKIPPED_FUNCTIONS,
            "if_not_else",
            "skipped-functions",
        )?;

        Ok(Self { skipped_functions })
    }
}
