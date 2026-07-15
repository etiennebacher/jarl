/// Default maximum number of positional arguments allowed in a call.
const DEFAULT_MAX_POSITIONAL_ARGS: usize = 2;

/// TOML options for `[lint.positional_arguments]`.
///
/// Use `max-positional-args` to control how many positional (unnamed) arguments
/// a call may have before it is reported. Defaults to `2`.
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PositionalArgumentsOptions {
    pub max_positional_args: Option<usize>,
}

/// Resolved options for the `positional_arguments` rule, ready for use during
/// linting.
#[derive(Clone, Debug)]
pub struct ResolvedPositionalArgumentsOptions {
    pub max_positional_args: usize,
}

impl ResolvedPositionalArgumentsOptions {
    pub fn resolve(options: Option<&PositionalArgumentsOptions>) -> anyhow::Result<Self> {
        let max_positional_args = options
            .and_then(|opts| opts.max_positional_args)
            .unwrap_or(DEFAULT_MAX_POSITIONAL_ARGS);

        Ok(Self { max_positional_args })
    }
}
