//! CLI entry point for the Flir Language Server
//!
//! This binary provides the `flir-lsp` command that starts the LSP server
//! for real-time diagnostic highlighting in editors and IDEs.
//!
//! This is a diagnostics-only LSP server - no formatting, code actions,
//! or other advanced features. Just highlighting lint issues as you type.

use anyhow::Result;
use clap::{Arg, Command};
use std::process;

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let matches = Command::new("flir-lsp")
        .version(flir_lsp::version())
        .about("Flir Language Server - Real-time diagnostics for your linter")
        .long_about(concat!(
            "Starts the Flir Language Server for real-time lint diagnostics in editors and IDEs.\n\n",
            "This server provides diagnostic highlighting only - no code actions, formatting, ",
            "or other advanced features. Connect your editor to this server via the LSP protocol ",
            "to get real-time feedback as you write code."
        ))
        .arg(
            Arg::new("log-level")
                .long("log-level")
                .value_name("LEVEL")
                .help("Set the logging level")
                .value_parser(["error", "warn", "info", "debug", "trace"])
                .default_value("info")
        )
        .arg(
            Arg::new("log-file")
                .long("log-file")
                .value_name("FILE")
                .help("Write logs to a file instead of stderr")
        )
        .get_matches();

    // Set up logging based on CLI arguments
    setup_logging(
        matches.get_one::<String>("log-level").unwrap(),
        matches.get_one::<String>("log-file"),
    )?;

    // Log startup information
    tracing::info!("Starting Flir LSP server v{}", flir_lsp::version());
    tracing::info!("Server mode: diagnostics only (no code actions or formatting)");
    tracing::info!("Communication: stdio");
    tracing::info!("Use Ctrl+C to stop the server");

    // Start the LSP server (always uses stdio)
    flir_lsp::run()
}

fn setup_logging(level: &str, log_file: Option<&String>) -> Result<()> {
    use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

    let filter = EnvFilter::try_new(format!("flir_lsp={}", level))
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    if let Some(log_file) = log_file {
        // Log to file - useful for debugging
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)?;

        tracing_subscriber::registry()
            .with(
                fmt::Layer::new()
                    .with_writer(file)
                    .with_ansi(false) // No colors in log files
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_line_number(true)
                    .with_thread_names(true),
            )
            .with(filter)
            .init();

        // Also log to stderr that we're logging to a file
        eprintln!("Flir LSP server logging to: {}", log_file);
    } else {
        // Log to stderr (IMPORTANT: never use stdout as it interferes with LSP protocol)
        tracing_subscriber::registry()
            .with(
                fmt::Layer::new()
                    .with_writer(std::io::stderr)
                    .with_ansi(true)
                    .with_target(false) // Less verbose for stderr
                    .with_line_number(false)
                    .with_thread_names(false),
            )
            .with(filter)
            .init();
    }

    Ok(())
}
