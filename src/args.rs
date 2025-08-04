use clap::{Parser, arg};

#[derive(Parser, Debug)]
#[command(
    author,
    name = "flir",
    about = "flir: Find and Fix Lints in R Code",
    after_help = "For help with a specific command, see: `flir help <command>`."
)]
pub struct CliArgs {
    #[arg(
        short,
        long,
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
}
