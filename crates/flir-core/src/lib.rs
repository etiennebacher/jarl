//! Core functionality for the flir R linter
//!
//! This crate provides the core linting functionality including:
//! - AST analysis and rule checking
//! - Diagnostic generation and reporting
//! - Configuration management
//! - File discovery and processing

pub mod analyze;
pub mod check;
pub mod config;
pub mod description;
pub mod diagnostic;
pub mod discovery;
pub mod error;
pub mod fix;
pub mod fs;
pub mod lints;
pub mod location;
pub mod rule_table;
pub mod settings;
pub mod toml;
pub mod utils;

#[cfg(test)]
pub mod test_utils;

// Re-export commonly used types for convenience
pub use check::check;
pub use config::{ArgsConfig, Config, build_config};
pub use diagnostic::Diagnostic;
pub use discovery::{DiscoveredSettings, discover_r_file_paths, discover_settings};
pub use location::Location;
pub use rule_table::RuleTable;
pub use settings::Settings;
