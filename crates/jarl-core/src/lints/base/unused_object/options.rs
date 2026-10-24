use std::collections::HashSet;

use crate::rule_options::resolve_with_extend;

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

/// <!-- docs: start -->
/// Use `skipped-functions` to fully replace the default list of calls whose
/// directly-assigned arguments are allowed to be unused. Use
/// `extend-skipped-functions` to add to the default list. Specifying both is an
/// error.
///
/// Function names in `skipped-functions` or `extend-skipped-functions` also match
/// namespaced calls, e.g. `skipped-functions = ["expect_error"]` will allow
/// `expect_error()` and `testthat::expect_error()`.
///
/// Only the direct argument position counts: an assignment nested in a block or in
/// a function defined inside the call is an ordinary local and is still reported.
///
/// ### Default values
///
/// ```toml
/// skipped-functions = [
///     "expect_error",         # from {testthat}
///     "expect_warning",       # from {testthat}
///     "expect_message",       # from {testthat}
///     "expect_silent",        # from {testthat}
///     "expect_defunct",       # from {lifecycle}
///     "expect_deprecated",    # from {lifecycle}
///     "expect_snapshot",      # from {testthat}
///     "expect_no_condition",  # from {testthat}
///     "expect_no_warning",    # from {testthat}
///     "expect_no_error",      # from {testthat}
///     "expect_no_message"     # from {testthat}
/// ]
/// ```
///
/// ### TOML settings
///
/// ```toml
/// [lint]
/// ...
///
/// [lint.unused_object]
/// # Also allow an unused assignment passed straight to `my_expect()`.
/// extend-skipped-functions = ["my_expect"]
/// ```
/// <!-- docs: end -->
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
