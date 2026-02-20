use air_r_parser::RParserOptions;
use air_r_syntax::{AnyRExpression, RBinaryExpressionFields, RSyntaxKind};
use biome_rowan::{AstNode, TextRange};
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
            let RBinaryExpressionFields { left, operator, .. } = binary.as_fields();

            let Ok(op) = operator else { continue };

            // Only <- (ASSIGN) and = (EQUAL) operators
            if !matches!(op.kind(), RSyntaxKind::ASSIGN | RSyntaxKind::EQUAL) {
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
pub fn compute_package_duplicate_assignments(
    paths: &[PathBuf],
) -> HashMap<PathBuf, Vec<(String, TextRange)>> {
    // Group files by package root.
    // We store pairs of (relativized_path, original_path) so that the map keys
    // match the `file` argument passed to `get_checks()`.
    let mut packages: HashMap<String, Vec<(PathBuf, PathBuf)>> = HashMap::new();

    for path in paths {
        // Rmd/Qmd files are excluded from this rule
        if crate::fs::has_rmd_extension(path) {
            continue;
        }
        // Only R files with .R extension inside a package R/ directory
        if !crate::fs::has_r_extension(path) {
            continue;
        }
        if let Some(root) = find_package_root(path) {
            let rel_path = PathBuf::from(crate::fs::relativize_path(path));
            // Use the string form of the relativized root as the grouping key
            let root_key = crate::fs::relativize_path(&root);
            packages
                .entry(root_key)
                .or_default()
                .push((rel_path, path.clone()));
        }
    }

    let mut result: HashMap<PathBuf, Vec<(String, TextRange)>> = HashMap::new();

    for (_root_key, mut file_pairs) in packages {
        // Sort alphabetically by the relativized path for deterministic ordering
        file_pairs.sort_by(|a, b| a.0.cmp(&b.0));

        // Track the first occurrence of each name across the whole package
        let mut seen: HashMap<String, ()> = HashMap::new();

        for (rel_path, orig_path) in &file_pairs {
            let assignments = collect_top_level_assignments(orig_path);
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
                result.insert(rel_path.clone(), file_duplicates);
            }
        }
    }

    result
}
