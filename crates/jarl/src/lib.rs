use crate::args::Args;
use crate::args::Command;
use crate::status::ExitStatus;

pub mod args;
pub mod commands;
pub mod logging;
pub mod output_format;
pub mod statistics;
pub mod status;

pub use args::CheckCommand;
pub use output_format::{ConciseEmitter, JsonEmitter, OutputFormat};

pub fn run(args: Args) -> anyhow::Result<ExitStatus> {
    if !matches!(args.command, Command::Server(_)) {
        // The language server sets up its own logging
        logging::init_logging(args.global_options.log_level.unwrap_or_default());
    }

    match args.command {
        Command::Check(command) => commands::check::check(command),
        Command::Server(command) => commands::server::server(command),
    }
}
