// Adapted from https://github.com/posit-dev/air/blob/main/crates/workspace/src/settings.rs#L30

// use air_workspace::settings::FormatSettings;

/// Resolved configuration settings used within air
#[derive(Debug, Default)]
pub struct Settings {
    pub linter: LinterSettings,
}

#[derive(Debug)]
pub struct LinterSettings {
    pub select: Option<Vec<String>>,
    pub ignore: Option<Vec<String>>,
    // pub exclude: Option<ExcludePatterns>,
}

impl Default for LinterSettings {
    /// [Default] handler for [LinterSettings]
    ///
    /// Uses `None` to indicate no rules specified, rather than empty vectors.
    fn default() -> Self {
        Self {
            select: None,
            ignore: None,
            // exclude: Default::default(),
        }
    }
}

// impl LinterSettings {
//     fn new(select: Option<Vec<String>>, ignore: Option<Vec<String>>) -> Self {
//         LinterSettings { select, ignore }
//     }
// }

// impl From<FormatSettings> for LinterSettings {
//     fn from(settings: FormatSettings) -> LinterSettings {
//         LinterSettings::new(Some(vec!["".to_string()]), Some(vec!["".to_string()]))
//     }
// }

// impl Settings {
//     fn new(settings: LinterSettings) -> Self {
//         Settings { linter: settings }
//     }
// }

// impl From<air_workspace::settings::Settings> for Settings {
//     fn from(settings: air_workspace::settings::Settings) -> Settings {
//         Settings::new(LinterSettings::from(settings.format))
//     }
// }
