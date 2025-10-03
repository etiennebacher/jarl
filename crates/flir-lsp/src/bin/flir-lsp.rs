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
    eprintln!("FLIR LSP Binary: main() started");

    if let Err(err) = run() {
        eprintln!("FLIR LSP Binary: run() failed with error: {err}");
        for cause in err.chain() {
            eprintln!("  Caused by: {cause}");
        }
        process::exit(1);
    }

    eprintln!("FLIR LSP Binary: main() completed successfully");
}

fn run() -> Result<()> {
    eprintln!("FLIR LSP Binary: run() called");

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
    eprintln!("FLIR LSP Binary: About to setup logging");
    setup_logging(
        matches.get_one::<String>("log-level").unwrap(),
        matches.get_one::<String>("log-file"),
    )?;
    eprintln!("FLIR LSP Binary: Logging setup completed");

    // Log startup information
    tracing::info!("Starting Flir LSP server v{}", flir_lsp::version());
    tracing::info!("Server mode: diagnostics only (no code actions or formatting)");
    tracing::info!("Communication: stdio");
    tracing::info!("Use Ctrl+C to stop the server");

    // Start the LSP server (always uses stdio)
    eprintln!("FLIR LSP Binary: About to call flir_lsp::run()");
    let result = flir_lsp::run();
    eprintln!(
        "FLIR LSP Binary: flir_lsp::run() completed with result: {:?}",
        result.is_ok()
    );
    result
}

fn setup_logging(level: &str, log_file: Option<&String>) -> Result<()> {
    eprintln!("FLIR LSP Binary: Setting up logging with level: {level}");

    use tracing_subscriber::{EnvFilter, fmt};

    // Simple, robust logging setup
    let filter = match EnvFilter::try_new(format!("flir_lsp={level}")) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("FLIR LSP Binary: Failed to parse log level, using 'info'");
            EnvFilter::try_new("info").unwrap_or_else(|_| EnvFilter::new(""))
        }
    };

    if let Some(log_file) = log_file {
        eprintln!("FLIR LSP Binary: Attempting to log to file: {log_file}");
        // Log to file - useful for debugging
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
        {
            Ok(file) => {
                fmt()
                    .with_writer(file)
                    .with_ansi(false)
                    .with_target(true)
                    .with_env_filter(filter)
                    .init();
                eprintln!("Flir LSP server logging to: {log_file}");
            }
            Err(e) => {
                eprintln!(
                    "FLIR LSP Binary: Failed to open log file, falling back to stderr: {e}"
                );
                fmt()
                    .with_writer(std::io::stderr)
                    .with_ansi(false)
                    .with_target(false)
                    .with_env_filter(filter)
                    .init();
            }
        }
    } else {
        eprintln!("FLIR LSP Binary: Logging to stderr");
        // Log to stderr (IMPORTANT: never use stdout as it interferes with LSP protocol)
        fmt()
            .with_writer(std::io::stderr)
            .with_ansi(false)
            .with_target(false)
            .with_env_filter(filter)
            .init();
    }

    eprintln!("FLIR LSP Binary: Logging setup completed");
    Ok(())
}
