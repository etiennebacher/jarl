// Adapted from https://github.com/posit-dev/air/blob/main/crates/workspace/src/settings.rs#L30

/// Resolved configuration settings used within flir
#[derive(Debug, Default)]
pub struct Settings {
    pub linter: LinterSettings,
}

#[derive(Debug)]
pub struct LinterSettings {
    pub select: Option<Vec<String>>,
    pub ignore: Option<Vec<String>>,
}

impl Default for LinterSettings {
    /// [Default] handler for [LinterSettings]
    ///
    /// Uses `None` to indicate no rules specified, rather than empty vectors.
    fn default() -> Self {
        Self { select: None, ignore: None }
    }
}
