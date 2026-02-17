//! Core functionality for the jarl R linter
//!
//! This crate provides the core linting functionality including:
//! - AST analysis and rule checking
//! - Diagnostic generation and reporting
//! - Configuration management
//! - File discovery and processing

#[macro_use]
pub mod utils;

pub mod analyze;
pub mod check;
pub mod config;
pub mod description;
pub mod diagnostic;
pub mod directive;
pub mod discovery;
pub mod error;
pub mod fix;
pub mod fs;
pub mod lints;
pub mod location;
pub mod rule_options;
pub mod rule_set;
pub mod settings;
pub mod suppression;
pub mod suppression_edit;
pub mod toml;
pub mod utils_ast;
pub mod vcs;

#[cfg(test)]
pub mod utils_test;
