use air_r_parser::RParserOptions;
use air_r_syntax::{AnyRExpression, RBinaryExpressionFields, RSyntaxKind};
use biome_rowan::{AstNode, TextRange};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Returns the package root directory if `file` is an R package source file.
///
/// A file qualifies if:
/// - Its immediate parent directory is named `"R"`
/// - A `DESCRIPTION` file exists in some ancestor directory
///
/// Returns `None` for Rmd/Qmd files or files not inside a package `R/` directory.
pub fn find_package_root(file: &Path) -> Option<PathBuf> {
    // The file's direct parent must be named "R"
    let parent = file.parent()?;
    let parent_name = parent.file_name()?.to_str()?;
    if parent_name != "R" {
        return None;
    }

    // Walk upwards from the grandparent looking for a DESCRIPTION file
    let mut current = parent.parent()?;
    loop {
        if current.join("DESCRIPTION").exists() {
            return Some(current.to_path_buf());
        }
        match current.parent() {
            Some(p) => current = p,
            None => return None,
        }
    }
}

/// Parse `file` and return `(name, lhs_range)` for each top-level `<-` or `=`
/// assignment whose left-hand side is a simple identifier.
///
/// Only top-level (not nested inside functions / blocks) expressions are
/// considered. `<<-` and right-assignment forms are excluded.
pub fn collect_top_level_assignments(file: &Path) -> Vec<(String, TextRange)> {
    let Ok(content) = std::fs::read_to_string(file) else {
        return vec![];
    };

    let parsed = air_r_parser::parse(&content, RParserOptions::default());
    if parsed.has_error() {
        return vec![];
    }

    let mut assignments = Vec::new();

    for expr in parsed.tree().expressions() {
        if let AnyRExpression::RBinaryExpression(binary) = expr {
            let RBinaryExpressionFields { left, operator, right } = binary.as_fields();

            let Ok(op) = operator else { continue };

            // Only <- (ASSIGN) and = (EQUAL) operators
            if !matches!(op.kind(), RSyntaxKind::ASSIGN | RSyntaxKind::EQUAL) {
                continue;
            }
            // Only flag assignments where the RHS is a function definition
            let Ok(rhs) = right else { continue };
            if rhs.as_r_function_definition().is_none() {
                continue;
            }

            let Ok(lhs) = left else { continue };

            // Only flag simple identifier targets
            if let AnyRExpression::RIdentifier(ident) = lhs {
                let name = ident.syntax().text_trimmed().to_string();
                let range = ident.syntax().text_trimmed_range();
                assignments.push((name, range));
            }
        }
    }

    assignments
}

/// Group R-package files by package root, detect names assigned more than once
/// (across files or within a file), and return a per-file map of
/// `(name, lhs_range)` pairs that should be flagged.
///
/// Files within each package are processed in alphabetical order by their
/// relativized path. The **first** occurrence of each name is never flagged;
/// all subsequent occurrences are flagged.
// (root_key, rel_path, assignments) per package file
type FileEntry = (String, PathBuf, Vec<(String, TextRange)>);
// per-package sorted list: (rel_path, assignments)
type PackageFiles = Vec<(PathBuf, Vec<(String, TextRange)>)>;

pub fn compute_package_duplicate_assignments(
    paths: &[PathBuf],
) -> HashMap<PathBuf, Vec<(String, TextRange)>> {
    // For each R-package file, resolve its package root and collect top-level
    // function assignments. .
    let file_data: Vec<FileEntry> = paths
        .par_iter()
        .filter(|p| !crate::fs::has_rmd_extension(p) && crate::fs::has_r_extension(p))
        .filter_map(|path| {
            let root = find_package_root(path)?;
            let rel_path = PathBuf::from(crate::fs::relativize_path(path));
            let root_key = crate::fs::relativize_path(&root);
            let assignments = collect_top_level_assignments(path);
            Some((root_key, rel_path, assignments))
        })
        .collect();

    // Group by package root, sort filesalphabetically, and detect duplicate names.
    let mut packages: HashMap<String, PackageFiles> = HashMap::new();
    for (root_key, rel_path, assignments) in file_data {
        packages
            .entry(root_key)
            .or_default()
            .push((rel_path, assignments));
    }

    let mut result: HashMap<PathBuf, Vec<(String, TextRange)>> = HashMap::new();

    for (_root_key, mut file_data) in packages {
        // Sort alphabetically by the relativized path for deterministic ordering
        file_data.sort_by(|a, b| a.0.cmp(&b.0));

        // Track the first occurrence of each name across the whole package
        let mut seen: HashMap<String, ()> = HashMap::new();

        for (rel_path, assignments) in file_data {
            let mut file_duplicates: Vec<(String, TextRange)> = Vec::new();

            for (name, range) in assignments {
                match seen.entry(name.clone()) {
                    std::collections::hash_map::Entry::Occupied(_) => {
                        // This is a duplicate: flag it
                        file_duplicates.push((name, range));
                    }
                    std::collections::hash_map::Entry::Vacant(e) => {
                        // First occurrence: record it, do not flag
                        e.insert(());
                    }
                }
            }

            if !file_duplicates.is_empty() {
                result.insert(rel_path, file_duplicates);
            }
        }
    }

    result
}
