use biome_rowan::TextRange;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::Config;
use crate::lints::base::duplicated_function_definition::duplicated_function_definition::compute_package_duplicate_assignments;
pub use crate::lints::base::duplicated_function_definition::duplicated_function_definition::is_in_r_package;
use crate::lints::base::unused_function::unused_function::compute_package_unused_functions;
use crate::rule_set::Rule;

/// Pre-computed cross-file analysis results for an R package.
///
/// Separated from `Config` so that user settings and analysis results
/// live in different structs (following Ruff's pattern).
#[derive(Clone, Debug, Default)]
pub struct PackageAnalysis {
    /// Per-file duplicate top-level assignment data.
    /// Keyed by relativized file path. Value is a list of `(name, lhs_range,
    /// help)` triples where `help` points to the first definition.
    pub duplicate_assignments: HashMap<PathBuf, Vec<(String, TextRange, String)>>,
    /// Per-file unused internal function data.
    /// Keyed by relativized file path. Value is a list of `(name, lhs_range,
    /// help)` triples for functions that are defined but never called and not
    /// exported.
    pub unused_functions: HashMap<PathBuf, Vec<(String, TextRange, String)>>,
}

/// Compute all package-level analysis for the given paths.
///
/// Pass `check_duplicates` / `check_unused` as `false` to skip the
/// corresponding (potentially expensive) cross-file scan.
pub fn compute_package_analysis(paths: &[PathBuf], config: &Config) -> PackageAnalysis {
    let duplicate_assignments = if config
        .rules_to_apply
        .contains(&Rule::DuplicatedFunctionDefinition)
    {
        compute_package_duplicate_assignments(paths)
    } else {
        HashMap::new()
    };

    let unused_functions = if config.rules_to_apply.contains(&Rule::UnusedFunction) {
        compute_package_unused_functions(paths, &config.rule_options.unused_function)
    } else {
        HashMap::new()
    };

    PackageAnalysis { duplicate_assignments, unused_functions }
}
