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
    #[serde(default, deserialize_with = "deserialize_max_positional_args")]
    pub max_positional_args: Option<usize>,
}

/// Deserialize `max-positional-args` while reporting a human-readable type in
/// error messages (`a non-negative integer`) instead of the Rust type `usize`.
fn deserialize_max_positional_args<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct MaxPositionalArgsVisitor;

    impl serde::de::Visitor<'_> for MaxPositionalArgsVisitor {
        type Value = Option<usize>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a non-negative integer")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(value as usize))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            usize::try_from(value).map(Some).map_err(|_| {
                E::custom(format!(
                    "invalid value: integer `{value}`, expected a non-negative integer"
                ))
            })
        }
    }

    deserializer.deserialize_any(MaxPositionalArgsVisitor)
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
