use std::collections::HashSet;

use crate::rule_options::resolve_with_extend;

/// Functions whose negated calls are allowed by default, e.g. `!is.null(x)`.
const DEFAULT_EXCEPTIONS: &[&str] = &["is.null", "is.na", "missing"];

/// TOML options for `[lint.if_not_else]`.
///
/// Use `exceptions` to fully replace the default list of functions whose negated
/// calls are allowed as an `if`/`ifelse()` condition. Use `extend-exceptions` to
/// add to the default list. Specifying both is an error.
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct IfNotElseOptions {
    pub exceptions: Option<Vec<String>>,
    pub extend_exceptions: Option<Vec<String>>,
}

/// Resolved options for the `if_not_else` rule, ready for use during linting.
#[derive(Clone, Debug)]
pub struct ResolvedIfNotElseOptions {
    pub exceptions: HashSet<String>,
}

impl ResolvedIfNotElseOptions {
    pub fn resolve(options: Option<&IfNotElseOptions>) -> anyhow::Result<Self> {
        let exceptions = resolve_with_extend(
            options.and_then(|opts| opts.exceptions.as_ref()),
            options.and_then(|opts| opts.extend_exceptions.as_ref()),
            DEFAULT_EXCEPTIONS,
            "if_not_else",
            "exceptions",
        )?;

        Ok(Self { exceptions })
    }
}
