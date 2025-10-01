//! Example of how to integrate Flir LSP into your existing CLI structure
//!
//! This shows how you might add an `lsp` subcommand to your main flir CLI
//! that starts the language server for real-time diagnostics in editors.

use anyhow::Result;
use clap::{Parser, Subcommand};

/// Main Flir CLI application
#[derive(Parser)]
#[command(name = "flir")]
#[command(about = "A fast linter for your favorite language")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check files for lint violations
    Check {
        /// Files or directories to check
        paths: Vec<String>,
        /// Fix violations automatically
        #[arg(long)]
        fix: bool,
        /// Show additional statistics
        #[arg(long)]
        stats: bool,
    },
    /// Start the Language Server Protocol server for real-time diagnostics
    Lsp {
        /// Set the logging level for the LSP server
        #[arg(long, default_value = "info")]
        log_level: String,
        /// Write logs to a file instead of stderr
        #[arg(long)]
        log_file: Option<String>,
        /// Maximum number of worker threads (defaults to CPU count, max 4)
        #[arg(long)]
        max_threads: Option<usize>,
    },
    /// Show configuration information
    Config {
        /// Show the configuration file path
        #[arg(long)]
        show_files: bool,
        /// Validate the current configuration
        #[arg(long)]
        validate: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { paths, fix, stats } => {
            // This would call your existing flir_cli functionality
            println!("Checking paths: {:?}", paths);
            println!("Fix mode: {}", fix);
            println!("Show stats: {}", stats);

            // Example of how you might call your existing linting logic:
            // use flir_cli::check;
            // let config = flir_core::Config::load_default()?;
            // let results = check::run_check(paths, config, fix)?;
            //
            // if stats {
            //     println!("Found {} issues", results.total_issues());
            // }
            //
            // if results.has_errors() {
            //     std::process::exit(1);
            // }

            Ok(())
        }
        Commands::Lsp {
            log_level,
            log_file,
            max_threads,
        } => {
            // Set up logging for the LSP server
            setup_lsp_logging(&log_level, log_file.as_deref())?;

            // Configure worker threads if specified
            if let Some(threads) = max_threads {
                std::env::set_var("FLIR_LSP_MAX_THREADS", threads.to_string());
            }

            // Show startup message
            println!("Starting Flir Language Server...");
            println!("Log level: {}", log_level);
            if let Some(ref log_file) = log_file {
                println!("Logging to: {}", log_file);
            } else {
                println!("Logging to stderr");
            }
            println!("Press Ctrl+C to stop the server");
            println!("Configure your editor to connect to this server for real-time diagnostics");

            // Start the LSP server
            flir_lsp::run()
        }
        Commands::Config {
            show_files,
            validate,
        } => {
            // This would call your existing config functionality
            println!(
                "Config - show_files: {}, validate: {}",
                show_files, validate
            );

            // Example of how you might handle configuration:
            // use flir_core::config;
            //
            // if show_files {
            //     let config_paths = config::find_config_files(".")?;
            //     for path in config_paths {
            //         println!("Config file: {}", path.display());
            //     }
            // }
            //
            // if validate {
            //     let config = config::Config::load_default()?;
            //     println!("Configuration is valid");
            //     println!("Enabled rules: {}", config.enabled_rules().len());
            // }

            Ok(())
        }
    }
}

fn setup_lsp_logging(level: &str, log_file: Option<&str>) -> Result<()> {
    use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

    let filter = EnvFilter::try_new(format!("flir_lsp={}", level))
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    if let Some(log_file) = log_file {
        // Log to file
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
                    .with_line_number(true),
            )
            .with(filter)
            .init();
    } else {
        // Log to stderr (important: not stdout, as that interferes with LSP protocol)
        tracing_subscriber::registry()
            .with(
                fmt::Layer::new()
                    .with_writer(std::io::stderr)
                    .with_ansi(true)
                    .with_target(false) // Less verbose for stderr
                    .with_line_number(false),
            )
            .with(filter)
            .init();
    }

    Ok(())
}

/// Example of how you might extend the LSP integration with configuration
#[derive(Debug, Clone)]
pub struct LspConfig {
    pub max_threads: Option<usize>,
    pub log_level: String,
    pub log_file: Option<String>,
    pub lint_on_open: bool,
    pub lint_on_change: bool,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            max_threads: None,
            log_level: "info".to_string(),
            log_file: None,
            lint_on_open: true,
            lint_on_change: true,
        }
    }
}

impl LspConfig {
    /// Load LSP configuration from environment variables
    pub fn load() -> Result<Self> {
        let mut config = Self::default();

        // Override with environment variables
        if let Ok(threads) = std::env::var("FLIR_LSP_MAX_THREADS") {
            config.max_threads = Some(threads.parse()?);
        }

        if let Ok(level) = std::env::var("FLIR_LSP_LOG_LEVEL") {
            config.log_level = level;
        }

        if let Ok(file) = std::env::var("FLIR_LSP_LOG_FILE") {
            config.log_file = Some(file);
        }

        Ok(config)
    }

    /// Apply this configuration to the LSP server startup
    pub fn apply(&self) -> Result<()> {
        if let Some(threads) = self.max_threads {
            std::env::set_var("FLIR_LSP_MAX_THREADS", threads.to_string());
        }

        setup_lsp_logging(&self.log_level, self.log_file.as_deref())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }

    #[test]
    fn test_lsp_config_default() {
        let config = LspConfig::default();
        assert_eq!(config.log_level, "info");
        assert!(config.lint_on_open);
        assert!(config.lint_on_change);
    }

    #[test]
    fn test_lsp_config_from_env() {
        std::env::set_var("FLIR_LSP_MAX_THREADS", "2");
        std::env::set_var("FLIR_LSP_LOG_LEVEL", "debug");

        let config = LspConfig::load().unwrap();
        assert_eq!(config.max_threads, Some(2));
        assert_eq!(config.log_level, "debug");

        // Clean up
        std::env::remove_var("FLIR_LSP_MAX_THREADS");
        std::env::remove_var("FLIR_LSP_LOG_LEVEL");
    }
}
