use crate::logging::LogLevel;
use crate::output_format::OutputFormat;
use clap::{Parser, Subcommand, arg};

#[derive(Parser)]
#[command(
    author,
    name = "jarl",
    about = "jarl: Find and Fix Lints in R Code",
    after_help = "For help with a specific command, see: `jarl help <command>`."
)]
#[command(version)]
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
        help = "Names of rules to include, separated by a comma (no spaces)."
    )]
    pub select_rules: String,
    #[arg(
        short,
        long,
        default_value = "",
        help = "Names of rules to exclude, separated by a comma (no spaces)."
    )]
    pub ignore_rules: String,
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

    /// Disable colored output. To turn colored output off, either set this option or set
    /// the environment variable `NO_COLOR` to any non-zero value.
    #[arg(long, global = true)]
    pub(crate) no_color: bool,
}
