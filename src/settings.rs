// Adapted from https://github.com/posit-dev/air/blob/main/crates/workspace/src/settings.rs#L30

use air_workspace::settings::DefaultExcludePatterns;
use air_workspace::settings::DefaultIncludePatterns;
use air_workspace::settings::ExcludePatterns;
use air_workspace::settings::FormatSettings;

/// Resolved configuration settings used within air
#[derive(Debug, Default)]
pub struct Settings {
    pub linter: LinterSettings,
}

#[derive(Debug)]
pub struct LinterSettings {
    pub exclude: Option<ExcludePatterns>,
    pub default_exclude: Option<DefaultExcludePatterns>,
    pub default_include: Option<DefaultIncludePatterns>,
}

impl Default for LinterSettings {
    /// [Default] handler for [LinterSettings]
    ///
    /// Notably:
    /// - `default_exclude` and `default_include` are `Some(<default>)` rather than `None`
    fn default() -> Self {
        Self {
            exclude: Default::default(),
            default_exclude: Some(Default::default()),
            default_include: Some(Default::default()),
        }
    }
}

impl LinterSettings {
    fn new(
        exclude: Option<ExcludePatterns>,
        default_exclude: Option<DefaultExcludePatterns>,
        default_include: Option<DefaultIncludePatterns>,
    ) -> Self {
        LinterSettings { exclude, default_exclude, default_include }
    }
}

impl From<FormatSettings> for LinterSettings {
    fn from(settings: FormatSettings) -> LinterSettings {
        LinterSettings::new(
            settings.exclude,
            settings.default_exclude,
            settings.default_include,
        )
    }
}

impl Settings {
    fn new(settings: LinterSettings) -> Self {
        Settings { linter: settings }
    }
}

impl From<air_workspace::settings::Settings> for Settings {
    fn from(settings: air_workspace::settings::Settings) -> Settings {
        Settings::new(LinterSettings::from(settings.format))
    }
}
