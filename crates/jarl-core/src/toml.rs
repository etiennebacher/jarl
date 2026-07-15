//
// Adapted from Ark
// https://github.com/posit-dev/air/blob/affa92cd514525c4bab6c8c2ca251ea19414b89f/crates/workspace/src/toml.rs
// and
// https://github.com/posit-dev/air/blob/affa92cd514525c4bab6c8c2ca251ea19414b89f/crates/workspace/src/toml_options.rs
//
// MIT License - Posit PBC

use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use crate::config::{get_invalid_rules, replace_group_rules, unknown_rules_error};
use crate::lints::base::assignment::options::AssignmentConfig;
use crate::lints::base::assignment::options::AssignmentOptions;
use crate::lints::base::duplicated_arguments::options::DuplicatedArgumentsOptions;
use crate::lints::base::if_not_else::options::IfNotElseOptions;
use crate::lints::base::implicit_assignment::options::ImplicitAssignmentOptions;
use crate::lints::base::missing_argument::options::MissingArgumentOptions;
use crate::lints::base::nested_pipe::options::NestedPipeOptions;
use crate::lints::base::pipe_consistency::options::PipeConsistencyOptions;
use crate::lints::base::positional_arguments::options::PositionalArgumentsOptions;
use crate::lints::base::quotes::options::QuotesOptions;
use crate::lints::base::true_false_symbol::options::TrueFalseSymbolOptions;
use crate::lints::base::undesirable_function::options::UndesirableFunctionOptions;
use crate::lints::base::unreachable_code::options::UnreachableCodeOptions;
use crate::lints::base::unused_function::options::UnusedFunctionOptions;
use crate::per_file_ignores::PerFileIgnores;
use crate::rule_options::ResolvedRuleOptions;
use crate::rule_set::Rule;
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

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TomlOptions {
    #[serde(flatten)]
    pub global: GlobalTomlOptions,
    pub lint: Option<LinterTomlOptions>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct GlobalTomlOptions {}

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
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

    /// # Patterns to include in checking
    ///
    /// By default, jarl checks all files with a `.R`, `.qmd`, `.Rmd`, or `.rmd`
    /// extension discovered in the provided paths. Use this option to restrict
    /// checking to files that match at least one of the supplied patterns. An
    /// empty list or a missing option means no restriction, i.e. all discovered
    /// files are checked.
    ///
    /// Include patterns follow the same format as `exclude` patterns (gitignore
    /// style, resolved relative to the `jarl.toml` directory). For example:
    ///
    /// - `R/` only checks files inside the `R/` directory.
    ///
    /// - `test-*.R` only checks files whose name matches `test-*.R`.
    ///
    /// - `**/*.{Rmd,qmd}` only checks Rmd and qmd files.
    ///
    /// When both `include` and `exclude` are specified, a file is checked only
    /// if it matches at least one `include` pattern and does not match any
    /// `exclude` pattern.
    pub include: Option<Vec<String>>,

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
    /// Jarl automatically excludes a default set of folders and files. If this option is
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

    /// # Per-file rule ignores
    ///
    /// A mapping of glob patterns to lists of rules that should be ignored in
    /// the files matching each pattern. Patterns are gitignore-style and
    /// resolved relative to the directory containing `jarl.toml` (the same
    /// format used by `include` and `exclude`). Rule names and rule groups
    /// (e.g. `PERF`) are both accepted.
    ///
    /// A pattern can be negated with a leading `!`, in which case its rules are
    /// ignored in every file that does *not* match the pattern. When several
    /// patterns match a file, the rules from all of them are ignored.
    ///
    /// For example:
    ///
    /// ```toml
    /// [lint.per-file-ignores]
    /// "foo.R" = ["true_false_symbol"]
    /// # ignore everywhere but in the R folder
    /// "!R/**.R" = ["any_is_na"]
    /// ```
    pub per_file_ignores: Option<HashMap<String, Vec<String>>>,

    /// # Whether to lint R code in roxygen `@examples` and `@examplesIf` sections
    ///
    /// When enabled, Jarl parses and checks R code found in roxygen2
    /// `@examples` and `@examplesIf` documentation sections. Only applies to
    /// files inside an R package (i.e. in the `R/` directory with a
    /// `DESCRIPTION` file in the parent).
    ///
    /// Defaults to `true`.
    pub check_roxygen: Option<bool>,

    /// # Whether to apply autofixes to roxygen examples
    ///
    /// When enabled, Jarl will attempt to apply fixes to R code inside
    /// roxygen2 `@examples` and `@examplesIf` sections. Since Air does not
    /// currently support formatting roxygen examples, this is opt-in.
    ///
    /// Defaults to `false`.
    pub fix_roxygen: Option<bool>,
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
    #[serde(rename = "duplicated_arguments")]
    pub duplicated_arguments: Option<DuplicatedArgumentsOptions>,

    /// # Options for the `if_not_else` rule
    ///
    /// Use `skipped-functions` to fully replace the default list of functions
    /// whose negated calls are allowed as an `if`/`ifelse()` condition. Use
    /// `extend-skipped-functions` to add to the default list.
    /// Specifying both is an error.
    #[serde(rename = "if_not_else")]
    pub if_not_else: Option<IfNotElseOptions>,

    /// # Options for the `implicit_assignment` rule
    ///
    /// Use `skipped-functions` to fully replace the default list of functions
    /// that are allowed to contain implicit assignment. Use
    /// `extend-skipped-functions` to add to the default list.
    /// Specifying both is an error.
    #[serde(rename = "implicit_assignment")]
    pub implicit_assignment: Option<ImplicitAssignmentOptions>,

    /// # Options for the `missing_argument` rule
    ///
    /// Use `skipped-functions` to fully replace the default list of functions
    /// whose empty arguments are allowed. Use `extend-skipped-functions` to
    /// add to the default list.
    /// Specifying both is an error.
    #[serde(rename = "missing_argument")]
    pub missing_argument: Option<MissingArgumentOptions>,

    /// # Options for the `nested_pipe` rule
    ///
    /// Use `skipped-functions` to fully replace the default list of outer calls
    /// whose nested pipes are allowed. Use `extend-skipped-functions` to add to
    /// the default list.
    /// Specifying both is an error.
    #[serde(rename = "nested_pipe")]
    pub nested_pipe: Option<NestedPipeOptions>,

    /// # Options for the `pipe_consistency` rule
    ///
    /// Use `preferred` to choose the preferred pipe operator. Valid values
    /// are `"|>"` (default) and `"%>%"`.
    #[serde(rename = "pipe_consistency")]
    pub pipe_consistency: Option<PipeConsistencyOptions>,

    /// # Options for the `positional_arguments` rule
    ///
    /// Use `max-positional-args` to control how many positional (unnamed)
    /// arguments a call may have before it is reported. Defaults to `2`.
    #[serde(rename = "positional_arguments")]
    pub positional_arguments: Option<PositionalArgumentsOptions>,

    /// # Options for the `quotes` rule
    ///
    /// Use `quote` to choose the preferred quote delimiter for string
    /// literals. Valid values are `"double"` (default) and `"single"`.
    #[serde(rename = "quotes")]
    pub quotes: Option<QuotesOptions>,

    /// # Options for the `true_false_symbol` rule
    ///
    /// Use `skipped-functions` to list functions whose arguments are allowed to
    /// contain the `T` and `F` symbols. This list is empty by default.
    #[serde(rename = "true_false_symbol")]
    pub true_false_symbol: Option<TrueFalseSymbolOptions>,

    /// # Options for the `undesirable_function` rule
    ///
    /// Use `functions` to fully replace the default list of undesirable functions.
    /// Use `extend-functions` to add to the default list.
    /// Specifying both is an error.
    #[serde(rename = "undesirable_function")]
    pub undesirable_function: Option<UndesirableFunctionOptions>,

    /// # Options for the `unreachable_code` rule
    ///
    /// Use `stopping-functions` to fully replace the default list of functions
    /// that are considered to stop execution (never return). Use
    /// `extend-stopping-functions` to add to the default list.
    /// Specifying both is an error.
    #[serde(rename = "unreachable_code")]
    pub unreachable_code: Option<UnreachableCodeOptions>,

    /// # Options for the `unused_function` rule
    ///
    /// Use `threshold-ignore` to control how many `unused_function`
    /// violations are allowed before they are all hidden (likely false
    /// positives).
    ///
    /// Use `skipped-functions` to determine which functions won't be reported
    /// even if Jarl considers them unused.
    #[serde(rename = "unused_function")]
    pub unused_function: Option<UnusedFunctionOptions>,

    /// Catch any unknown fields so we can produce a clean error message that
    /// only lists the primary `[lint]` options (not every rule sub-table).
    #[serde(flatten)]
    #[cfg_attr(feature = "schemars", schemars(skip))]
    pub(crate) unknown_fields: HashMap<String, toml::Value>,
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
    pub fn into_settings(self, root: &Path) -> anyhow::Result<Settings> {
        let linter = self.lint.unwrap_or_default();

        // Reject unknown fields in `[lint]` with a clean error message that
        // only lists the primary options (not every rule sub-table name).
        if let Some(field) = linter.unknown_fields.keys().next() {
            return Err(anyhow::anyhow!(
                "Unknown field `{field}` in `[lint]`. Expected one of: \
                 `select`, `extend-select`, `ignore`, `fixable`, `unfixable`, \
                 `exclude`, `default-exclude`, `include`, `per-file-ignores`, \
                 `check-roxygen`, `fix-roxygen`."
            ));
        }

        let per_file_ignores = resolve_per_file_ignores(linter.per_file_ignores.as_ref(), root)?;

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
            include: linter.include,
            exclude: linter.exclude,
            default_exclude: linter.default_exclude,
            check_roxygen: linter.check_roxygen,
            fix_roxygen: linter.fix_roxygen,
            fixable: linter.fixable,
            unfixable: linter.unfixable,
            deprecated_assignment_syntax,
            rule_options: ResolvedRuleOptions::resolve(
                assignment_options.as_ref(),
                linter.duplicated_arguments.as_ref(),
                linter.if_not_else.as_ref(),
                linter.implicit_assignment.as_ref(),
                linter.missing_argument.as_ref(),
                linter.nested_pipe.as_ref(),
                linter.pipe_consistency.as_ref(),
                linter.positional_arguments.as_ref(),
                linter.quotes.as_ref(),
                linter.true_false_symbol.as_ref(),
                linter.undesirable_function.as_ref(),
                linter.unreachable_code.as_ref(),
                linter.unused_function.as_ref(),
            )?,
            per_file_ignores,
        };

        Ok(Settings { linter })
    }
}

/// Validate and compile the `[lint.per-file-ignores]` map into a
/// [PerFileIgnores], expanding rule groups and checking rule names just like
/// `select`/`ignore`.
fn resolve_per_file_ignores(
    per_file_ignores: Option<&HashMap<String, Vec<String>>>,
    root: &Path,
) -> anyhow::Result<PerFileIgnores> {
    let Some(per_file_ignores) = per_file_ignores else {
        return Ok(PerFileIgnores::default());
    };

    let all_rules = Rule::all();
    let mut entries = Vec::with_capacity(per_file_ignores.len());

    for (pattern, rule_names) in per_file_ignores {
        let passed_by_user = rule_names.iter().map(|s| s.as_str()).collect();
        let expanded_rules = replace_group_rules(&passed_by_user, all_rules);
        if let Some(invalid) = get_invalid_rules(all_rules, &expanded_rules) {
            return Err(unknown_rules_error(
                format!(
                    "Unknown rules in `per-file-ignores` for pattern '{}': {}",
                    pattern,
                    invalid.names.join(", ")
                ),
                invalid.help,
            ));
        }
        let rules: Vec<Rule> = expanded_rules
            .iter()
            .filter_map(|name| Rule::from_name(name))
            .collect();
        entries.push((pattern.clone(), rules));
    }

    PerFileIgnores::new(root, entries)
}
