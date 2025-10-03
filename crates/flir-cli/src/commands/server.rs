// use crate::args::LanguageServerCommand;
use crate::{args::ServerCommand, status::ExitStatus};

pub(crate) fn server(_command: ServerCommand) -> anyhow::Result<ExitStatus> {
    eprintln!("FLIR CLI: Starting server command");

    match flir_lsp::run() {
        Ok(()) => {
            eprintln!("FLIR CLI: LSP server completed successfully");
            Ok(ExitStatus::Success)
        }
        Err(e) => {
            eprintln!("FLIR CLI: LSP server failed with error: {e}");
            for cause in e.chain() {
                eprintln!("  Caused by: {cause}");
            }
            Err(e)
        }
    }
}
