use std::collections::HashSet;

use super::resolve_with_extend;

/// Default functions where implicit assignments are allowed.
const DEFAULT_SKIPPED_FUNCTIONS: &[&str] = &[
    "expect_error",
    "expect_warning",
    "expect_message",
    "expect_snapshot",
    "quote",
    "suppressMessages",
    "suppressWarnings",
];

/// TOML options for `[lint.implicit_assignment]`.
///
/// Use `skipped-functions` to fully replace the default list of functions
/// where implicit assignments are allowed. Use
/// `extend-skipped-functions` to add to the default list.
/// Specifying both is an error.
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ImplicitAssignmentOptions {
    pub skipped_functions: Option<Vec<String>>,
    pub extend_skipped_functions: Option<Vec<String>>,
}

/// Resolved options for the `implicit_assignment` rule, ready for use during
/// linting.
#[derive(Clone, Debug)]
pub struct ResolvedImplicitAssignmentOptions {
    pub skipped_functions: HashSet<String>,
}

impl ResolvedImplicitAssignmentOptions {
    pub fn resolve(options: Option<&ImplicitAssignmentOptions>) -> anyhow::Result<Self> {
        let (base, extend) = match options {
            Some(opts) => (
                opts.skipped_functions.as_ref(),
                opts.extend_skipped_functions.as_ref(),
            ),
            None => (None, None),
        };

        let skipped_functions = resolve_with_extend(
            base,
            extend,
            DEFAULT_SKIPPED_FUNCTIONS,
            "implicit_assignment",
            "skipped-functions",
        )?;

        Ok(Self { skipped_functions })
    }
}
