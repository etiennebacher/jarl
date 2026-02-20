use biome_rowan::{TextRange, TextSize};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn is_in_r_package(file: &Path) -> Option<bool> {
    // The file's direct parent must be named "R"
    let parent = file.parent()?;
    let parent_name = parent.file_name()?.to_str()?;
    if parent_name != "R" {
        return Some(false);
    }

    // At this point, the parent is "R" so we search for "DESCRIPTION" in its
    // parent folder.
    Some(parent.parent()?.join("DESCRIPTION").exists())
}

/// Fast line-based scan for top-level function assignments.
///
/// Avoids a full R parse, making this roughly 10× cheaper. Used in the
/// pre-computation step so each file is only fully parsed once (in the main
/// lint pass).
///
/// Limitation: misses the rare pattern where `function` is on a different line
/// than `<-` (e.g. `foo <-\n  function(...)`).
pub fn scan_top_level_assignments(content: &str) -> Vec<(String, TextRange, u32, u32)> {
    let mut results = Vec::new();
    let mut byte_offset: usize = 0;
    let mut line_no: u32 = 1;

    for line_with_ending in content.split_inclusive('\n') {
        // Strip \r\n or \n to get the line without its terminator
        let line = line_with_ending
            .trim_end_matches('\n')
            .trim_end_matches('\r');
        let trimmed = line.trim_start();
        let leading = line.len() - trimmed.len();

        // Only look at unindented lines. In R packages, all top-level function
        // definitions are at column 1. Indented lines are inside function bodies
        // or control-flow blocks and must not be collected.
        if leading == 0 && !trimmed.is_empty() && !trimmed.starts_with('#') {
            // Parse the leading identifier (R: alphanumeric + '.' + '_')
            let ident_end = trimmed
                .find(|c: char| !c.is_alphanumeric() && c != '.' && c != '_')
                .unwrap_or(trimmed.len());

            if ident_end > 0 {
                let name = &trimmed[..ident_end];
                let after_ident = trimmed[ident_end..].trim_start();

                // Match <- or = (but not ==)
                let after_op = if let Some(rest) = after_ident.strip_prefix("<-") {
                    Some(rest.trim_start())
                } else if after_ident.starts_with('=') && !after_ident.starts_with("==") {
                    Some(after_ident[1..].trim_start())
                } else {
                    None
                };

                if let Some(rhs) = after_op {
                    // `function` keyword (not `functionalities` etc.) or `\` lambda
                    let is_function = rhs.starts_with("function")
                        && rhs[8..]
                            .chars()
                            .next()
                            .is_none_or(|c| !c.is_alphanumeric() && c != '.' && c != '_');
                    let is_lambda = rhs.starts_with('\\');

                    if is_function || is_lambda {
                        let lhs_start = byte_offset + leading;
                        let lhs_end = lhs_start + ident_end;
                        let range = TextRange::new(
                            TextSize::from(lhs_start as u32),
                            TextSize::from(lhs_end as u32),
                        );
                        results.push((name.to_string(), range, line_no, (leading + 1) as u32));
                    }
                }
            }
        }

        byte_offset += line_with_ending.len();
        line_no += 1;
    }

    results
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
    // function assignments.
    let file_data: Vec<FileEntry> = paths
        .par_iter()
        .filter(|p| crate::fs::has_r_extension(p))
        .filter(|p| is_in_r_package(p).unwrap_or(false))
        .filter_map(|path| {
            let root = path.parent()?;
            let rel_path = PathBuf::from(crate::fs::relativize_path(path));
            let root_key = crate::fs::relativize_path(root);
            // Use the fast text scan — the full parser runs later in the main
            // lint pass so each file is only parsed once.
            let content = std::fs::read_to_string(path).ok()?;
            let assignments = scan_top_level_assignments(&content);
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
