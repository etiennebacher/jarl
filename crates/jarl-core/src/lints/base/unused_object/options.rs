use std::collections::HashSet;

use crate::rule_options::resolve_with_extend;

/// Default calls whose directly-assigned arguments are allowed to be unused.
///
/// These `testthat` expectations run their argument for the condition it
/// signals — or for a snapshot of its output — rather than for its value, so an
/// assignment handed straight to one of them exists to give the expectation
/// something to evaluate rather than to bind a value someone later reads.
const DEFAULT_SKIPPED_FUNCTIONS: &[&str] = &[
    "expect_error",
    "expect_warning",
    "expect_message",
    "expect_silent",
    "expect_defunct",
    "expect_deprecated",
    "expect_snapshot",
    "expect_no_condition",
    "expect_no_warning",
    "expect_no_error",
    "expect_no_message",
];

/// TOML options for `[lint.unused_object]`.
///
/// Use `skipped-functions` to fully replace the default list of calls whose
/// directly-assigned arguments are allowed to be unused. Use
/// `extend-skipped-functions` to add to the default list. Specifying both is an
/// error.
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct UnusedObjectOptions {
    pub skipped_functions: Option<Vec<String>>,
    pub extend_skipped_functions: Option<Vec<String>>,
}

/// Resolved options for the `unused_object` rule, ready for use during linting.
#[derive(Clone, Debug)]
pub struct ResolvedUnusedObjectOptions {
    pub skipped_functions: HashSet<String>,
}

impl ResolvedUnusedObjectOptions {
    pub fn resolve(options: Option<&UnusedObjectOptions>) -> anyhow::Result<Self> {
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
            "unused_object",
            "skipped-functions",
        )?;

        Ok(Self { skipped_functions })
    }
}
