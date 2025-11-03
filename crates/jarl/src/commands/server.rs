// use crate::args::LanguageServerCommand;
use crate::{args::ServerCommand, status::ExitStatus};

pub(crate) fn server(_command: ServerCommand) -> anyhow::Result<ExitStatus> {
    eprintln!("JARL CLI: Starting server command");

    match jarl_lsp::run() {
        Ok(()) => {
            eprintln!("JARL CLI: LSP server completed successfully");
            Ok(ExitStatus::Success)
        }
        Err(e) => {
            eprintln!("JARL CLI: LSP server failed with error: {e}");
            for cause in e.chain() {
                eprintln!("  Caused by: {cause}");
            }
            Err(e)
        }
    }
}
