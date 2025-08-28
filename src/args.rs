use clap::{Parser, arg};

use crate::output_format::OutputFormat;

#[derive(Parser, Debug)]
#[command(
    author,
    name = "flir",
    about = "flir: Find and Fix Lints in R Code",
    after_help = "For help with a specific command, see: `flir help <command>`."
)]
pub struct CliArgs {
    #[arg(
        default_value = ".",
        help = "The directory in which to check or fix lints."
    )]
    pub dir: String,
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
        short,
        long,
        default_value = "",
        help = "Names of rules to include, separated by a comma (no spaces)."
    )]
    pub rules: String,
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
