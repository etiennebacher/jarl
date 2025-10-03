//! # Flir Language Server Protocol Implementation
//!
//! A minimal LSP server focused purely on providing real-time lint diagnostics.
//! This implementation handles document management and diagnostic publishing
//! without any code actions, formatting, or other advanced features.

use anyhow::{Context, Result};
use std::num::NonZeroUsize;

pub use client::Client;
pub use document::{DocumentKey, PositionEncoding, TextDocument};
pub use server::Server;
pub use session::{DocumentSnapshot, Session};

mod client;
mod document;
mod lint;
mod server;
mod session;

#[allow(dead_code)]
pub(crate) const SERVER_NAME: &str = "flir";
pub(crate) const DIAGNOSTIC_SOURCE: &str = "Flir";

/// Common result type used throughout the LSP implementation
pub(crate) type LspResult<T> = anyhow::Result<T>;

/// Main entry point for running the Flir LSP server
///
/// This function sets up a minimal LSP server that provides real-time
/// lint diagnostics as you type in your editor.
pub fn run() -> Result<()> {
    eprintln!("FLIR LSP: run() function called, setting up server");
    tracing::info!("Starting Flir Language Server v{}", version());

    // Set up worker threads for background linting
    let worker_threads = std::thread::available_parallelism()
        .unwrap_or(NonZeroUsize::new(2).unwrap())
        .min(NonZeroUsize::new(4).unwrap());

    tracing::info!("Using {} worker threads for linting", worker_threads);
    eprintln!("FLIR LSP: Creating stdio connection");

    // Create LSP connection over stdio
    let (connection, io_threads) = lsp_server::Connection::stdio();
    eprintln!("FLIR LSP: stdio connection created successfully");

    // Start the server
    eprintln!("FLIR LSP: Creating server instance");
    let server =
        Server::new(worker_threads, connection).context("Failed to create Flir LSP server")?;

    eprintln!("FLIR LSP: Starting server.run()");
    let server_result = server.run();
    eprintln!(
        "FLIR LSP: server.run() completed with result: {:?}",
        server_result.is_ok()
    );

    // Wait for IO threads to complete
    eprintln!("FLIR LSP: Waiting for IO threads to complete");
    let io_result = io_threads.join();
    eprintln!(
        "FLIR LSP: IO threads completed with result: {:?}",
        io_result.is_ok()
    );

    // Handle results
    match (server_result, io_result) {
        (Ok(()), Ok(())) => {
            eprintln!("FLIR LSP: Server shut down successfully");
            tracing::info!("Flir LSP server shut down successfully");
            Ok(())
        }
        (Err(server_err), Err(io_err)) => {
            eprintln!(
                "FLIR LSP: Server error: {server_err}, IO error: {io_err}"
            );
            tracing::error!("Server error: {}, IO error: {}", server_err, io_err);
            Err(server_err).context(format!("IO thread error: {io_err}"))
        }
        (Err(server_err), _) => {
            eprintln!("FLIR LSP: Server error: {server_err}");
            tracing::error!("Server error: {}", server_err);
            Err(server_err)
        }
        (_, Err(io_err)) => {
            eprintln!("FLIR LSP: IO error: {io_err}");
            tracing::error!("IO error: {}", io_err);
            Err(io_err).context("IO thread error")
        }
    }
}

/// Returns the version of the Flir LSP server
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }

    #[test]
    fn test_constants() {
        assert_eq!(SERVER_NAME, "flir");
        assert_eq!(DIAGNOSTIC_SOURCE, "Flir");
    }
}
