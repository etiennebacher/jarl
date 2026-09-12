use std::collections::HashSet;

use crate::rule_options::resolve_with_extend;

/// Default operators that are considered undesirable.
const DEFAULT_OPERATORS: &[&str] = &["->>", ":::", "<<-"];

/// TOML options for `[lint.undesirable_operator]`.
///
/// Use `operators` to fully replace the default list of undesirable operators.
/// Use `extend-operators` to add to the default list.
/// Specifying both is an error.
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct UndesirableOperatorOptions {
    pub operators: Option<Vec<String>>,
    pub extend_operators: Option<Vec<String>>,
}

/// Resolved options for the `undesirable_operator` rule, ready for use during
/// linting.
#[derive(Clone, Debug)]
pub struct ResolvedUndesirableOperatorOptions {
    pub operators: HashSet<String>,
}

impl ResolvedUndesirableOperatorOptions {
    pub fn resolve(options: Option<&UndesirableOperatorOptions>) -> anyhow::Result<Self> {
        let (base, extend) = match options {
            Some(opts) => (opts.operators.as_ref(), opts.extend_operators.as_ref()),
            None => (None, None),
        };

        let operators = resolve_with_extend(
            base,
            extend,
            DEFAULT_OPERATORS,
            "undesirable_operator",
            "operators",
        )?;
        Ok(Self { operators })
    }
}
