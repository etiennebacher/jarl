// use crate::args::LanguageServerCommand;
use crate::{args::ServerCommand, status::ExitStatus};

pub(crate) fn server(_command: ServerCommand) -> anyhow::Result<ExitStatus> {
    flir_lsp::run();
    Ok(ExitStatus::Success)
}
