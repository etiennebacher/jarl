use air_r_syntax::RSyntaxKind;

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

/// Resolved options for the `assignment` rule, ready for use during linting.
#[derive(Clone, Debug)]
pub struct ResolvedAssignmentOptions {
    pub operator: RSyntaxKind,
}

impl ResolvedAssignmentOptions {
    pub fn resolve(options: Option<&AssignmentOptions>) -> anyhow::Result<Self> {
        let operator = match options.and_then(|opts| opts.operator.as_deref()) {
            Some("<-") | None => RSyntaxKind::ASSIGN,
            Some("=") => RSyntaxKind::EQUAL,
            Some(other) => {
                return Err(anyhow::anyhow!(
                    "Invalid value for `operator` in `[lint.assignment]`: \"{other}\". \
                     Expected \"<-\" or \"=\"."
                ));
            }
        };

        Ok(Self { operator })
    }
}
