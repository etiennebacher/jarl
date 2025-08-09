use crate::check_expression::*;
use crate::config::Config;
use crate::fix::*;
use crate::message::*;

use anyhow::{Context, Result};
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

pub fn check(config: Config) -> Result<Vec<Diagnostic>, anyhow::Error> {
    let result: Result<Vec<Diagnostic>, anyhow::Error> = config
        .paths
        .par_iter()
        .map(|file| check_path(file, config.clone()))
        .flat_map(|result| match result {
            Ok(checks) => checks.into_par_iter().map(Ok).collect::<Vec<_>>(),
            Err(e) => vec![Err(e)],
        })
        .collect();

    result
}

pub fn check_path(path: &PathBuf, config: Config) -> Result<Vec<Diagnostic>, anyhow::Error> {
    if config.should_fix {
        lint_fix(path, config)
    } else {
        lint_only(path, config)
    }
}

pub fn lint_only(path: &PathBuf, config: Config) -> Result<Vec<Diagnostic>, anyhow::Error> {
    let contents = fs::read_to_string(Path::new(path))
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    let checks = get_checks(&contents, path, config.clone())
        .with_context(|| format!("Failed to get checks for file: {}", path.display()))?;

    Ok(checks)
}
pub fn lint_fix(path: &PathBuf, config: Config) -> Result<Vec<Diagnostic>, anyhow::Error> {
    let mut has_skipped_fixes = true;
    let mut checks: Vec<Diagnostic>;

    loop {
        let contents = fs::read_to_string(Path::new(path))
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        checks = get_checks(&contents, path, config.clone())
            .with_context(|| format!("Failed to get checks for file: {}", path.display()))?;

        if !has_skipped_fixes {
            break;
        }

        let (new_has_skipped_fixes, fixed_text) = apply_fixes(&checks, &contents);
        has_skipped_fixes = new_has_skipped_fixes;

        fs::write(path, fixed_text)
            .with_context(|| format!("Failed to write file: {}", path.display()))?;
    }

    Ok(checks)
}
