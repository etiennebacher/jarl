use crate::args::Args;
use crate::args::Command;
use crate::status::ExitStatus;

pub mod args;
pub mod commands;
pub mod logging;
pub mod output_format;
pub mod status;

pub use args::CheckCommand;
pub use output_format::{ConciseEmitter, JsonEmitter, OutputFormat};

pub fn run(args: Args) -> anyhow::Result<ExitStatus> {
    // Check both the --no-color flag and the NO_COLOR environment variable
    let no_color = args.global_options.no_color || std::env::var("NO_COLOR").is_ok();

    if !matches!(args.command, Command::Server(_)) {
        // The language server sets up its own logging
        logging::init_logging(args.global_options.log_level.unwrap_or_default(), no_color);
    }

    match args.command {
        Command::Check(command) => commands::check::check(command, no_color),
        Command::Server(command) => commands::server::server(command),
    }
}
