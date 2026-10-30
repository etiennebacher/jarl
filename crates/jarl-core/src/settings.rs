//
// Adapted from Ark
// https://github.com/posit-dev/air/blob/main/crates/workspace/src/settings.rs
//
// MIT License - Posit PBC

use crate::per_file_ignores::PerFileIgnores;
use crate::rule_options::ResolvedRuleOptions;

/// Resolved configuration settings used within jarl
#[derive(Clone, Debug, Default)]
pub struct Settings {
    pub linter: LinterSettings,
}

/// Uses `None` to indicate no rules specified, rather than empty vectors.
#[derive(Clone, Debug, Default)]
pub struct LinterSettings {
    pub select: Option<Vec<String>>,
    pub extend_select: Option<Vec<String>>,
    pub ignore: Option<Vec<String>>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub default_exclude: Option<bool>,
    pub check_roxygen: Option<bool>,
    pub fix_roxygen: Option<bool>,
    pub fixable: Option<Vec<String>>,
    pub unfixable: Option<Vec<String>>,
    pub rule_options: ResolvedRuleOptions,
    /// Per-file rule ignores resolved from `[lint.per-file-ignores]`.
    pub per_file_ignores: PerFileIgnores,
}
