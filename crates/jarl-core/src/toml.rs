//
// Adapted from Ark
// https://github.com/posit-dev/air/blob/affa92cd514525c4bab6c8c2ca251ea19414b89f/crates/workspace/src/toml.rs
// and
// https://github.com/posit-dev/air/blob/affa92cd514525c4bab6c8c2ca251ea19414b89f/crates/workspace/src/toml_options.rs
//
// MIT License - Posit PBC

use std::fmt::Display;
use std::fmt::Formatter;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use crate::rule_options::ResolvedRuleOptions;
use crate::rule_options::assignment::AssignmentConfig;
use crate::rule_options::assignment::AssignmentOptions;
use crate::rule_options::duplicated_arguments::DuplicatedArgumentsOptions;
use crate::rule_options::unreachable_code::UnreachableCodeOptions;
use crate::settings::LinterSettings;
use crate::settings::Settings;

#[derive(Debug)]
pub enum ParseTomlError {
    Read(PathBuf, io::Error),
    Deserialize(PathBuf, toml::de::Error),
}

impl std::error::Error for ParseTomlError {}

impl Display for ParseTomlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            // It's nicer if we don't make these paths relative, so we can quickly
            // jump to the TOML file to see what is wrong
            Self::Read(path, err) => {
                write!(f, "Failed to read {path}:\n{err}", path = path.display())
            }
            Self::Deserialize(path, err) => {
                write!(f, "Failed to parse {path}:\n{err}", path = path.display())
            }
        }
    }
}

pub fn parse_jarl_toml(path: &Path) -> Result<TomlOptions, ParseTomlError> {
    let toml = fs::read_to_string(path).unwrap();
    toml::from_str(&toml).map_err(|err| ParseTomlError::Deserialize(path.to_path_buf(), err))
}

#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct TomlOptions {
    #[serde(flatten)]
    pub global: GlobalTomlOptions,
    pub lint: Option<LinterTomlOptions>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct GlobalTomlOptions {}

#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct LinterTomlOptions {
    /// # Rules to select
    ///
    /// If this is empty, then all rules that are provided by `jarl` are used,
    /// with one limitation related to the minimum R version used in the project.
    /// By default, if this minimum R version is unknown, then all rules that
    /// have a version restriction are deactivated. This is for example the case
    /// of `grepv` since the eponymous function was introduced in R 4.5.0.
    ///
    /// There are three ways to inform `jarl` about the minimum version used in
    /// the project:
    /// 1. pass the argument `--min-r-version` in the CLI, e.g.,
    ///    `jarl --min-r-version 4.3`;
    /// 2. if the project is an R package, then `jarl` looks for mentions of a
    ///    minimum R version in the `Depends` field sometimes present in the
    ///    `DESCRIPTION` file.
    /// 3. specify `min-r-version` in `jarl.toml`.
    pub select: Option<Vec<String>>,

    /// # Additional rules to select
    ///
    /// This is a list of rule names to add on top of the existing selection.
    /// This is useful in the case where you want to use the default set of
    /// rules *and* some additional opt-in rules. In this scenario, you only
    /// need to add `extend-select = ["OPT_IN_RULE"]` instead of writing all
    /// default rule names.
    ///
    /// This has the same constraints as `select`.
    pub extend_select: Option<Vec<String>>,

    /// # Rules to ignore
    ///
    /// If this is empty, then no rules are excluded. This field has higher
    /// importance than `select`, so if a rule name appears by mistake in both
    /// `select` and `ignore`, it is ignored.
    pub ignore: Option<Vec<String>>,

    /// # Rule violations to always fix
    ///
    /// A list of rules for which violations will be fixed if possible. By
    /// default, all rules are considered fixable.
    /// This only matters if you pass `--fix` in the CLI.
    pub fixable: Option<Vec<String>>,

    /// # Rule violations to never fix
    ///
    /// A list of rules that are never fixed. This only matters if you pass
    /// `--fix` in the CLI.
    pub unfixable: Option<Vec<String>>,

    /// # Patterns to exclude from checking
    ///
    /// By default, jarl will refuse to check files matched by patterns listed in
    /// `default-exclude`. Use this option to supply an additional list of exclude
    /// patterns.
    ///
    /// Exclude patterns are modeled after what you can provide in a
    /// [.gitignore](https://git-scm.com/docs/gitignore), and are resolved relative to the
    /// parent directory that your `jarl.toml` is contained within. For example, if your
    /// `jarl.toml` was located at `root/jarl.toml`, then:
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
    ///   directory, where the `folder/` directory itself can appear anywhere.
    ///
    /// See the full [.gitignore](https://git-scm.com/docs/gitignore) documentation for
    /// all of the patterns you can provide.
    pub exclude: Option<Vec<String>>,

    /// # Whether or not to use default exclude patterns
    ///
    /// jarl automatically excludes a default set of folders and files. If this option is
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
    /// # Assignment operator to use
    ///
    /// Accepts either the legacy form `assignment = "<-"` (deprecated) or the
    /// new table form `[lint.assignment]` with an `operator` field.
    pub assignment: Option<AssignmentConfig>,

    /// # Options for the `duplicated_arguments` rule
    ///
    /// Use `skipped-functions` to fully replace the default list of functions
    /// that are allowed to have duplicated arguments. Use
    /// `extend-skipped-functions` to add to the default list.
    /// Specifying both is an error.
    pub duplicated_arguments: Option<DuplicatedArgumentsOptions>,

    /// # Options for the `unreachable_code` rule
    ///
    /// Use `stopping-functions` to fully replace the default list of functions
    /// that are considered to stop execution (never return). Use
    /// `extend-stopping-functions` to add to the default list.
    /// Specifying both is an error.
    pub unreachable_code: Option<UnreachableCodeOptions>,
}

/// Return the path to the `jarl.toml` or `.jarl.toml` file in a given directory.
pub fn find_jarl_toml_in_directory<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
    // Check for `jarl.toml` first, as we prioritize the "visible" one.
    let toml = path.as_ref().join("jarl.toml");
    if toml.is_file() {
        return Some(toml);
    }

    // Now check for `.jarl.toml` as well
    let toml = path.as_ref().join(".jarl.toml");
    if toml.is_file() {
        return Some(toml);
    }

    // Didn't find a configuration file
    None
}

/// Find the path to the closest `jarl.toml` or `.jarl.toml` if one exists, walking up the filesystem
pub fn find_jarl_toml<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
    for directory in path.as_ref().ancestors() {
        if let Some(toml) = find_jarl_toml_in_directory(directory) {
            return Some(toml);
        }
    }
    None
}

impl TomlOptions {
    pub fn into_settings(self, _root: &Path) -> anyhow::Result<Settings> {
        let linter = self.lint.unwrap_or_default();

        // Resolve the assignment config: extract the AssignmentOptions and
        // track whether the deprecated top-level string form was used.
        let (assignment_options, deprecated_assignment_syntax) = match &linter.assignment {
            Some(AssignmentConfig::Legacy(value)) => (
                Some(AssignmentOptions { operator: Some(value.clone()) }),
                true,
            ),
            Some(AssignmentConfig::Options(opts)) => (Some(opts.clone()), false),
            None => (None, false),
        };

        let linter = LinterSettings {
            select: linter.select,
            extend_select: linter.extend_select,
            ignore: linter.ignore,
            exclude: linter.exclude,
            default_exclude: linter.default_exclude,
            fixable: linter.fixable,
            unfixable: linter.unfixable,
            deprecated_assignment_syntax,
            rule_options: ResolvedRuleOptions::resolve(
                assignment_options.as_ref(),
                linter.duplicated_arguments.as_ref(),
                linter.unreachable_code.as_ref(),
            )?,
        };

        Ok(Settings { linter })
    }
}
