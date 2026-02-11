//
// Adapted from Ark
// https://github.com/posit-dev/air/blob/main/crates/workspace/src/settings.rs
//
// MIT License - Posit PBC

/// Resolved configuration settings used within jarl
#[derive(Clone, Debug, Default)]
pub struct Settings {
    pub linter: LinterSettings,
}

#[derive(Clone, Debug)]
pub struct LinterSettings {
    pub select: Option<Vec<String>>,
    pub extend_select: Option<Vec<String>>,
    pub ignore: Option<Vec<String>>,
    pub assignment: Option<String>,
    pub exclude: Option<Vec<String>>,
    pub default_exclude: Option<bool>,
    pub fixable: Option<Vec<String>>,
    pub unfixable: Option<Vec<String>>,
}

impl Default for LinterSettings {
    /// [Default] handler for [LinterSettings]
    ///
    /// Uses `None` to indicate no rules specified, rather than empty vectors.
    fn default() -> Self {
        Self {
            select: None,
            extend_select: None,
            ignore: None,
            assignment: None,
            exclude: None,
            default_exclude: None,
            fixable: None,
            unfixable: None,
        }
    }
}
