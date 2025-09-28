//! Command-line interface for the flir R linter
//!
//! This crate provides the CLI application that wraps the core flir functionality.

pub mod args;
pub mod output_format;

pub use args::CliArgs;
pub use output_format::{ConciseEmitter, JsonEmitter, OutputFormat};
