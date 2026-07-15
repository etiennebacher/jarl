use std::collections::HashSet;

use crate::rule_options::resolve_with_extend;

/// Default maximum number of positional arguments allowed in a call.
const DEFAULT_MAX_POSITIONAL_ARGS: usize = 3;

/// Variadic functions whose positional arguments are idiomatic and therefore
/// allowed by default, e.g. `c(1, 2, 3)` or `paste("a", "b", "c")`.
const DEFAULT_SKIPPED_FUNCTIONS: &[&str] = &[
    // in base R
    "c",
    "cat",
    "file.path",
    "gsub",
    "ifelse",
    "lapply",
    "list",
    "paste",
    "paste0",
    "sprintf",
    "switch",
    // in packages
    "fifelse",
    "if_else",
    "tribble",
];

/// TOML options for `[lint.positional_arguments]`.
///
/// Use `max-positional-args` to control how many positional (unnamed) arguments
/// a call may have before it is reported. Defaults to `2`.
///
/// Use `skipped-functions` to fully replace the default list of functions whose
/// positional arguments are allowed. Use `extend-skipped-functions` to add to
/// the default list. Specifying both is an error.
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PositionalArgumentsOptions {
    #[serde(default, deserialize_with = "deserialize_max_positional_args")]
    pub max_positional_args: Option<usize>,
    pub skipped_functions: Option<Vec<String>>,
    pub extend_skipped_functions: Option<Vec<String>>,
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
    pub skipped_functions: HashSet<String>,
}

impl ResolvedPositionalArgumentsOptions {
    pub fn resolve(options: Option<&PositionalArgumentsOptions>) -> anyhow::Result<Self> {
        let max_positional_args = options
            .and_then(|opts| opts.max_positional_args)
            .unwrap_or(DEFAULT_MAX_POSITIONAL_ARGS);

        let skipped_functions = resolve_with_extend(
            options.and_then(|opts| opts.skipped_functions.as_ref()),
            options.and_then(|opts| opts.extend_skipped_functions.as_ref()),
            DEFAULT_SKIPPED_FUNCTIONS,
            "positional_arguments",
            "skipped-functions",
        )?;

        Ok(Self { max_positional_args, skipped_functions })
    }
}
