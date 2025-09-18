use air_workspace::settings::DefaultExcludePatterns;
use air_workspace::settings::DefaultIncludePatterns;
use air_workspace::settings::ExcludePatterns;
use std::fs;
use std::io;
use std::path::PathBuf;
use toml::Table;
use toml::from_str;

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
    /// # Rules to select
    ///
    /// If this is empty, then all rules that are provided by `flir` are used,
    /// with one limitation related to the minimum R version used in the project.
    /// By default, if this minimum R version is unknown, then all rules that
    /// have a version restriction are deactivated. This is for example the case
    /// of `grepv` since the eponymous function was introduced in R 4.5.0.
    ///
    /// There are three ways to inform `flir` about the minimum version used in
    /// the project:
    /// 1. pass the argument `--min-r-version` in the CLI, e.g.,
    ///    `flir --min-r-version 4.3`;
    /// 2. if the project is an R package, then `flir` looks for mentions of a
    ///    minimum R version in the `Depends` field sometimes present in the
    ///    `DESCRIPTION` file.
    /// 3. specify `min-r-version` in `flir.toml`.
    pub select: Option<Vec<String>>,

    /// # Rules to ignore
    ///
    /// If this is empty, then no rules are excluded. This field has higher
    /// importance than `select`, so if a rule name appears by mistake in both
    /// `select` and `ignore`, it is ignored.
    pub ignore: Option<Vec<String>>,

    // TODO: Ruff also has a "fixable" field, but not sure what's the purpose
    // https://docs.astral.sh/ruff/configuration/#__tabbed_1_2
    /// # Rules for which the fix is never applied
    ///
    /// This only matters if you pass `--fix` in the CLI.
    pub unfixable: Option<Vec<String>>,

    /// # Patterns to exclude from checking
    ///
    /// By default, flir will refuse to check files matched by patterns listed in
    /// `default-exclude`. Use this option to supply an additional list of exclude
    /// patterns.
    ///
    /// Exclude patterns are modeled after what you can provide in a
    /// [.gitignore](https://git-scm.com/docs/gitignore), and are resolved relative to the
    /// parent directory that your `flir.toml` is contained within. For example, if your
    /// `flir.toml` was located at `root/flir.toml`, then:
    ///
    /// - `file.R` excludes a file named `file.R` located anywhere below `root/`. This is
    ///   equivalent to `**/file.R`.
    ///
    /// - `folder/` excludes a directory named `folder` (and all of its children) located
    ///   anywhere below `root/`. You can also just use `folder`, but this would
    ///   technically also match a file named `folder`, so the trailing slash is preferred
    ///   when targeting directories. This is equivalent to `**/folder/`.
    ///
    /// - `/file.R` excludes a file named `file.R` located at `root/file.R`.
    ///
    /// - `/folder/` excludes a directory named `folder` (and all of its children) located
    ///   at `root/folder/`.
    ///
    /// - `file-*.R` excludes R files named like `file-this.R` and `file-that.R` located
    ///   anywhere below `root/`.
    ///
    /// - `folder/*.R` excludes all R files located at `root/folder/`. Note that R files
    ///   in directories under `folder/` are not excluded in this case (such as
    ///   `root/folder/subfolder/file.R`).
    ///
    /// - `folder/**/*.R` excludes all R files located anywhere below `root/folder/`.
    ///
    /// - `**/folder/*.R` excludes all R files located directly inside a `folder/`
    ///   directory, where the `folder/` directory itself can /// appear anywhere.
    ///
    /// See the full [.gitignore](https://git-scm.com/docs/gitignore) documentation for
    /// all of the patterns you can provide.
    pub exclude: Option<Vec<String>>,

    /// # Whether or not to use default exclude patterns
    ///
    /// flir automatically excludes a default set of folders and files. If this option is
    /// set to `false`, these files will be formatted as well.
    ///
    /// The default set of excluded patterns are:
    /// - `.git/`
    /// - `renv/`
    /// - `revdep/`
    /// - `cpp11.R`
    /// - `RcppExports.R`
    /// - `extendr-wrappers.R`
    /// - `import-standalone-*.R`
    pub default_exclude: Option<bool>,
}
