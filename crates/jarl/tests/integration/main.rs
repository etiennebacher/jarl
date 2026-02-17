/// Integration tests for the Jarl CLI
///
/// Directory structure inspired by:
/// https://matklad.github.io/2021/02/27/delete-cargo-integration-tests.html
///
/// Resolves problems with:
/// - Compilation times, by only having 1 integration test binary
/// - Dead code analysis of integration test helpers https://github.com/rust-lang/rust/issues/46379
mod add_jarl_ignore;
mod allow_dirty;
mod allow_no_vcs;
mod assignment;
mod comments;
mod help;
mod helpers;
mod jarl;
mod min_r_version;
mod no_default_exclude;
mod output_format;
mod rules;
mod statistics;
mod toml;
mod toml_hierarchical;
mod toml_rule_args;
