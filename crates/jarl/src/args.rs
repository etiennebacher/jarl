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
    Check(CheckCommand),

    /// Start a language server
    Server(ServerCommand),
}

#[derive(Clone, Debug, Parser)]
#[command(arg_required_else_help(true))]
pub struct CheckCommand {
    #[arg(
        required = true,
        help = "List of files or directories to check or fix lints, for example `jarl check .`."
    )]
    pub files: Vec<String>,
    #[arg(
        short,
        long,
        default_value = "false",
        help = "Automatically fix issues detected by the linter."
    )]
    pub fix: bool,
    #[arg(
        short,
        long,
        default_value = "false",
        help = "Include fixes that may not retain the original intent of the  code."
    )]
    pub unsafe_fixes: bool,
    #[arg(
        long,
        default_value = "false",
        help = "Apply fixes to resolve lint violations, but don't report on leftover violations. Implies `--fix`."
    )]
    pub fix_only: bool,
    #[arg(
        long,
        default_value = "false",
        help = "Apply fixes even if the Git branch is not clean, meaning that there are uncommitted files."
    )]
    pub allow_dirty: bool,
    #[arg(
        long,
        default_value = "false",
        help = "Apply fixes even if there is no version control system."
    )]
    pub allow_no_vcs: bool,
    #[arg(
        short,
        long,
        default_value = "",
        help = "Names of rules to include, separated by a comma (no spaces). This also accepts names of groups of rules, such as \"PERF\"."
    )]
    pub select: String,
    #[arg(
        short,
        long,
        default_value = "",
        help = "Like `--select` but adds additional rules in addition to those already specified."
    )]
    pub extend_select: String,
    #[arg(
        short,
        long,
        default_value = "",
        help = "Names of rules to exclude, separated by a comma (no spaces). This also accepts names of groups of rules, such as \"PERF\"."
    )]
    pub ignore: String,
    #[arg(
        short,
        long,
        default_value = "false",
        help = "Show the time taken by the function."
    )]
    pub with_timing: bool,
    #[arg(
        short,
        long,
        help = "The mimimum R version to be used by the linter. Some rules only work starting from a specific version."
    )]
    pub min_r_version: Option<String>,
    #[arg(
        long,
        value_enum,
        default_value_t = OutputFormat::default(),
        help="Output serialization format for violations."
    )]
    pub output_format: OutputFormat,
    #[arg(
        long,
        value_enum,
        help = "Assignment operator to use, can be either `<-` or `=`."
    )]
    pub assignment: Option<String>,
    #[arg(
        long,
        default_value = "false",
        help = "Do not apply the default set of file patterns that should be excluded."
    )]
    pub no_default_exclude: bool,
    #[arg(
        long,
        default_value = "false",
        help = "Show counts for every rule with at least one violation."
    )]
    pub statistics: bool,
    #[arg(
        long,
        value_name = "REASON",
        default_missing_value = "<reason>",
        num_args = 0..=1,
        require_equals = true,
        help = "Automatically insert a `# jarl-ignore` comment to suppress all violations.\nThe default reason can be customized with `--add-jarl-ignore=\"my_reason\"`."
    )]
    pub add_jarl_ignore: Option<String>,
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
