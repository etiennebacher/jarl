use air_workspace::settings::DefaultExcludePatterns;
use air_workspace::settings::DefaultIncludePatterns;
use air_workspace::settings::ExcludePatterns;
use std::fs;
use std::io;
use std::path::PathBuf;
use toml::from_str;
use toml::Table;

use crate::settings::LinterSettings;

pub fn parse_flir_toml(dir: &PathBuf) -> Result<TomlOptions, ParseTomlError> {
    let path = dir.join("flir.toml");
    let path2 = path.to_path_buf();
    let toml = fs::read_to_string(path).unwrap();
    toml::from_str(&toml).map_err(|err| ParseTomlError::Deserialize(path2, err))
}

#[derive(Debug)]
pub enum ParseTomlError {
    Read(PathBuf, io::Error),
    Deserialize(PathBuf, toml::de::Error),
}

#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct TomlOptions {
    #[serde(flatten)]
    pub global: GlobalTomlOptions,
    pub linter: Option<LinterTomlOptions>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct GlobalTomlOptions {}

#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct LinterTomlOptions {
    pub rules: Option<Vec<String>>,
}

// impl TomlOptions {
//     pub fn into_settings(self, root: &Path) -> anyhow::Result<Settings> {
//         let linter = self.linter.unwrap_or_default();

//         let linter = LinterSettings {
//             exclude: Some(ExcludePatterns::default()),
//             default_exclude: Some(DefaultExcludePatterns::default()),
//             default_include: Some(DefaultIncludePatterns::default()),
//             rules: linter.rules,
//         };

//         Ok(Settings { linter })
//     }
// }
