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

/// Convert a byte offset within `content` to a 1-based `(line, col)` pair.
fn byte_offset_to_line_col(content: &str, offset: usize) -> (u32, u32) {
    let prefix = &content[..offset];
    let line = prefix.chars().filter(|&c| c == '\n').count() as u32 + 1;
    let col_start = prefix.rfind('\n').map(|n| n + 1).unwrap_or(0);
    let col = (offset - col_start) as u32 + 1;
    (line, col)
}

/// Parse `file` and return `(name, lhs_range, line, col)` for each top-level
/// `<-` or `=` assignment whose left-hand side is a simple identifier.
///
/// Only top-level (not nested inside functions / blocks) expressions are
/// considered. `<<-` and right-assignment forms are excluded.
/// `line` and `col` are 1-based positions of the LHS identifier.
pub fn collect_top_level_assignments(file: &Path) -> Vec<(String, TextRange, u32, u32)> {
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
                let offset: usize = range.start().into();
                let (line, col) = byte_offset_to_line_col(&content, offset);
                assignments.push((name, range, line, col));
            }
        }
    }

    assignments
}

/// Group R-package files by package root, detect names assigned more than once
/// (across files or within a file), and return a per-file map of
/// `(name, lhs_range, help)` triples that should be flagged.
///
/// `help` is a human-readable string indicating where the first definition
/// was found, e.g. `"other definition at R/aaa.R:1:1"`.
///
/// Files within each package are processed in alphabetical order by their
/// relativized path. The **first** occurrence of each name is never flagged;
/// all subsequent occurrences are flagged.
// (root_key, rel_path, assignments) per package file
type FileEntry = (String, PathBuf, Vec<(String, TextRange, u32, u32)>);
// per-package sorted list: (rel_path, assignments)
type PackageFiles = Vec<(PathBuf, Vec<(String, TextRange, u32, u32)>)>;

pub fn compute_package_duplicate_assignments(
    paths: &[PathBuf],
) -> HashMap<PathBuf, Vec<(String, TextRange, String)>> {
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

    let mut result: HashMap<PathBuf, Vec<(String, TextRange, String)>> = HashMap::new();

    for (_root_key, mut file_data) in packages {
        // Sort alphabetically by the relativized path for deterministic ordering
        file_data.sort_by(|a, b| a.0.cmp(&b.0));

        // Track the first occurrence of each name: (file, line, col)
        let mut seen: HashMap<String, (PathBuf, u32, u32)> = HashMap::new();

        for (rel_path, assignments) in file_data {
            let mut file_duplicates: Vec<(String, TextRange, String)> = Vec::new();

            for (name, range, line, col) in assignments {
                match seen.entry(name.clone()) {
                    std::collections::hash_map::Entry::Occupied(e) => {
                        // This is a duplicate: flag it with a pointer to the first definition
                        let (first_file, first_line, first_col) = e.get();
                        let help = format!(
                            "other definition at {}:{first_line}:{first_col}",
                            first_file.display()
                        );
                        file_duplicates.push((name, range, help));
                    }
                    std::collections::hash_map::Entry::Vacant(e) => {
                        // First occurrence: record its location, do not flag
                        e.insert((rel_path.clone(), line, col));
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
