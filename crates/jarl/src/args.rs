use crate::logging::LogLevel;
use crate::output_format::OutputFormat;
use clap::builder::Styles;
use clap::builder::styling::{AnsiColor, Effects};
use clap::{Parser, Subcommand};

// Configures Clap v3-style help menu colors
const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser)]
#[command(
    author,
    name = "jarl",
    about = "jarl: Just Another R Linter",
    after_help = "For help with a specific command, see: `jarl help <command>`."
)]
#[command(version)]
#[command(styles = STYLES)]
pub struct Args {
    #[command(subcommand)]
    pub(crate) command: Command,
    #[clap(flatten)]
    pub(crate) global_options: GlobalOptions,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Check a set of files or directories
    Check(Box<CheckCommand>),

    /// Print the documentation of a rule
    Rule(RuleCommand),

    /// Start a language server
    Server(ServerCommand),
}

#[derive(Clone, Debug, Parser)]
#[command(arg_required_else_help(true), disable_help_flag = true)]
pub struct CheckCommand {
    #[arg(
        required = true,
        help = "List of files or directories to check or fix lints, for example `jarl check .`."
    )]
    pub files: Vec<String>,
    #[arg(
        long,
        value_name = "FILES",
        value_delimiter = ',',
        require_equals = true,
        help_heading = "File selection",
        help = "List of file patterns to exclude from linting, separated by a comma (no spaces). Must be passed with an equals sign, e.g. `--exclude=R/*.R`, so the shell does not expand glob patterns."
    )]
    pub exclude: Vec<String>,
    #[arg(
        long,
        default_value = "false",
        help_heading = "File selection",
        help = "Do not apply the default set of file patterns that should be excluded."
    )]
    pub no_default_exclude: bool,
    #[arg(
        short,
        long,
        value_name = "RULES",
        default_value = "",
        help_heading = "Rule selection",
        help = "Names of rules to include, separated by a comma (no spaces). This also accepts names of groups of rules, such as \"PERF\"."
    )]
    pub select: String,
    #[arg(
        short,
        long,
        value_name = "RULES",
        default_value = "",
        help_heading = "Rule selection",
        help = "Like `--select` but adds additional rules in addition to those already specified."
    )]
    pub extend_select: String,
    #[arg(
        short,
        long,
        value_name = "RULES",
        default_value = "",
        help_heading = "Rule selection",
        help = "Names of rules to exclude, separated by a comma (no spaces). This also accepts names of groups of rules, such as \"PERF\"."
    )]
    pub ignore: String,
    #[arg(
        short,
        long,
        default_value = "false",
        help_heading = "Other options",
        help = "Automatically fix issues detected by the linter."
    )]
    pub fix: bool,
    #[arg(
        short,
        long,
        default_value = "false",
        help_heading = "Other options",
        help = "Include fixes that may not retain the original intent of the  code."
    )]
    pub unsafe_fixes: bool,
    #[arg(
        long,
        default_value = "false",
        help_heading = "Other options",
        help = "Apply fixes to resolve lint violations, but don't report on leftover violations. Implies `--fix`."
    )]
    pub fix_only: bool,
    #[arg(
        long,
        default_value = "false",
        help_heading = "Other options",
        help = "Apply fixes even if the Git branch is not clean, meaning that there are uncommitted files."
    )]
    pub allow_dirty: bool,
    #[arg(
        long,
        default_value = "false",
        help_heading = "Other options",
        help = "Apply fixes even if there is no version control system."
    )]
    pub allow_no_vcs: bool,
    #[arg(
        short,
        long,
        default_value = "false",
        help_heading = "Other options",
        help = "Show the time taken by the function."
    )]
    pub with_timing: bool,
    #[arg(
        short,
        long,
        help_heading = "Other options",
        help = "The mimimum R version to be used by the linter. Some rules only work starting from a specific version."
    )]
    pub min_r_version: Option<String>,
    #[arg(
        long,
        value_enum,
        default_value_t = OutputFormat::default(),
        help_heading = "Other options",
        help="Output serialization format for violations."
    )]
    pub output_format: OutputFormat,
    #[arg(
        long,
        value_enum,
        help_heading = "Other options",
        help = "[DEPRECATED: use `[lint.assignment]` in jarl.toml] Assignment operator to use, can be either `<-` or `=`."
    )]
    pub assignment: Option<String>,
    #[arg(
        long,
        default_value = "false",
        conflicts_with = "fix",
        conflicts_with = "unsafe_fixes",
        conflicts_with = "fix_only",
        help_heading = "Other options",
        help = "Show counts for every rule with at least one violation."
    )]
    pub statistics: bool,
    #[arg(
        long,
        value_name = "REASON",
        default_missing_value = "<reason>",
        num_args = 0..=1,
        require_equals = true,
        conflicts_with = "statistics",
        conflicts_with = "fix",
        conflicts_with = "unsafe_fixes",
        conflicts_with = "fix_only",
        help_heading = "Other options",
        help = "Automatically insert a `# jarl-ignore` comment to suppress all violations.\nThe default reason can be customized with `--add-jarl-ignore=\"my_reason\"`."
    )]
    pub add_jarl_ignore: Option<String>,
    // Help flag declared manually (auto flag disabled above) so it lands in the
    // "Other options" group instead of clap's default "Options" heading, which
    // would otherwise be forced to the top of the help output.
    #[arg(
        short,
        long,
        action = clap::ArgAction::Help,
        help_heading = "Other options",
        help = "Print help (see a summary with '-h')"
    )]
    pub help: Option<bool>,
}
#[derive(Clone, Debug, Parser)]
#[command(arg_required_else_help(true))]
pub struct RuleCommand {
    #[arg(
        required = true,
        help = "Name of the rule to explain, for example `jarl rule all_equal`."
    )]
    pub name: String,
}

#[derive(Clone, Debug, Parser)]
pub(crate) struct ServerCommand {}

/// All configuration options that can be passed "globally"
#[derive(Debug, Default, clap::Args)]
#[command(next_help_heading = "Global options")]
pub(crate) struct GlobalOptions {
    /// The log level. One of: `error`, `warn`, `info`, `debug`, or `trace`. Defaults
    /// to `warn`.
    #[arg(long, global = true)]
    pub(crate) log_level: Option<LogLevel>,
}
