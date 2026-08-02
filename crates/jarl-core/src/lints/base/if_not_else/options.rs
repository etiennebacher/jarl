use std::collections::HashSet;

use crate::rule_options::resolve_with_extend;

/// Functions whose negated calls are allowed by default, e.g. `!is.null(x)`.
const DEFAULT_SKIPPED_FUNCTIONS: &[&str] = &["is.null", "is.na", "missing"];

/// TOML options for `[lint.if_not_else]`.
///
/// Use `skipped-functions` to fully replace the default list of functions whose
/// negated calls are allowed as an `if`/`ifelse()` condition. Use
/// `extend-skipped-functions` to add to the default list. Specifying both is an
/// error.
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
