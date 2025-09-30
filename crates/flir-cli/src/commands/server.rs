// use crate::args::LanguageServerCommand;
use crate::{args::ServerCommand, status::ExitStatus};

// #[tokio::main]
// pub(crate) async fn server(_command: LanguageServerCommand) -> anyhow::Result<ExitStatus> {
//     // Returns after shutdown
//     lsp::start_lsp(tokio::io::stdin(), tokio::io::stdout()).await;

//     Ok(ExitStatus::Success)
// }
pub(crate) fn server(_command: ServerCommand) -> anyhow::Result<ExitStatus> {
    // Returns after shutdown
    // lsp::start_lsp(tokio::io::stdin(), tokio::io::stdout()).await;

    Ok(ExitStatus::Success)
}
