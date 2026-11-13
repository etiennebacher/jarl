use air_r_syntax::RSyntaxKind;

/// <!-- docs: start -->
/// The `lint.assignment.operator` option in `jarl.toml` must be set for this
/// rule to be checked. It takes a single value (`"<-"` or `"="`) indicating
/// the preferred assignment operator in the files to check.
/// If `operator = "<-"` then any use of the `"="` operator to assign values
/// will be reported, and vice-versa.
///
/// ### Default values
///
/// This option doesn't have a default value.
///
/// ### TOML settings
///
/// ```toml
/// [lint]
/// ...
///
/// [lint.assignment]
/// operator = "<-" # or "="
/// ```
/// <!-- docs: end -->
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
