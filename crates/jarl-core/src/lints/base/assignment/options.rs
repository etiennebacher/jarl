use air_r_syntax::RSyntaxKind;
use serde::Deserialize;

/// TOML options for `[lint.assignment]`.
///
/// Use `operator` to specify which assignment operator to enforce.
/// Valid values are `"<-"` (the default) and `"="`.
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct AssignmentOptions {
    pub operator: Option<String>,
}

/// Accepts either the legacy top-level string (`assignment = "<-"`) or the new
/// table form (`[lint.assignment]` with an `operator` field).
///
/// TOML doesn't allow a key to be both a string and a table, but by trying the
/// table form first we can surface clear errors (like "unknown field") while
/// still falling back to the legacy string form.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum AssignmentConfig {
    /// Legacy: `assignment = "<-"` (deprecated)
    Legacy(String),
    /// New: `[lint.assignment]` table with fields
    Options(AssignmentOptions),
}

impl<'de> serde::Deserialize<'de> for AssignmentConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(AssignmentConfigVisitor)
    }
}

/// Visitor that dispatches directly to the original deserializer so that error
/// positions (e.g. "line 3, column 1") point at the offending field rather
/// than the table header.
struct AssignmentConfigVisitor;

impl<'de> serde::de::Visitor<'de> for AssignmentConfigVisitor {
    type Value = AssignmentConfig;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a string (e.g. `assignment = \"<-\"`) or a table (e.g. `[lint.assignment]`)")
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(AssignmentConfig::Legacy(value.to_string()))
    }

    fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        let opts =
            AssignmentOptions::deserialize(serde::de::value::MapAccessDeserializer::new(map))?;
        Ok(AssignmentConfig::Options(opts))
    }
}

/// Resolved options for the `assignment` rule, ready for use during linting.
#[derive(Clone, Debug)]
pub struct ResolvedAssignmentOptions {
    pub operator: RSyntaxKind,
}

impl ResolvedAssignmentOptions {
    pub fn resolve(options: Option<&AssignmentOptions>) -> anyhow::Result<Self> {
        let operator = match options {
            Some(opts) => match opts.operator.as_deref() {
                Some("<-") | None => RSyntaxKind::ASSIGN,
                Some("=") => RSyntaxKind::EQUAL,
                Some(other) => {
                    return Err(anyhow::anyhow!(
                        "Invalid value for `operator` in `[lint.assignment]`: \"{other}\". \
                         Expected \"<-\" or \"=\"."
                    ));
                }
            },
            None => RSyntaxKind::ASSIGN,
        };

        Ok(Self { operator })
    }
}
