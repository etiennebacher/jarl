use biome_rowan::{TextRange, TextSize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::package::SharedFileData;

/// ## What it does
///
/// Checks for duplicated function definitions in R packages.
///
/// ## Why is this bad?
///
/// Having two functions with the same name is likely an error since development
/// tools such as `devtools::load_all()` will only load one of them. This rule
/// looks for function definitions shared across files in the same R package,
/// meaning files that are in a folder named "R" whose parent folder has a
/// `DESCRIPTION` file.
///
/// This rule doesn't have an automatic fix.
///
/// ## Example
///
/// ```r
/// # In "R/foo1.R":
/// foo <- function(x) {
///   x + 1
/// }
///
/// # In "R/foo2.R":
/// foo <- function(x) {
///   x + 2
/// }
///
/// # Function "foo" is defined in two different scripts in the same package,
/// # which is likely due to a mistake.
/// ```
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

/// Compute duplicate assignments from pre-scanned shared file data.
///
/// This is the inner logic extracted from `compute_package_duplicate_assignments`,
/// operating on already-scanned `SharedFileData` to avoid redundant file reads.
pub(crate) fn compute_duplicates_from_shared(
    shared_data: &[SharedFileData],
) -> HashMap<PathBuf, Vec<(String, TextRange, String)>> {
    // Group by package root
    let mut packages: HashMap<&str, Vec<&SharedFileData>> = HashMap::new();
    for fd in shared_data {
        packages.entry(&fd.root_key).or_default().push(fd);
    }

    let mut result: HashMap<PathBuf, Vec<(String, TextRange, String)>> = HashMap::new();

    for (_root_key, mut file_data) in packages {
        // Sort alphabetically by the relativized path for deterministic ordering
        file_data.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

        // Track the first occurrence of each name: (file, line, col)
        let mut seen: HashMap<&str, (&PathBuf, u32, u32)> = HashMap::new();

        for fd in &file_data {
            let mut file_duplicates: Vec<(String, TextRange, String)> = Vec::new();

            for (name, range, line, col) in &fd.assignments {
                match seen.entry(name.as_str()) {
                    std::collections::hash_map::Entry::Occupied(e) => {
                        let (first_file, first_line, first_col) = e.get();
                        let help = format!(
                            "Other definition at {}:{first_line}:{first_col}",
                            first_file.display()
                        );
                        file_duplicates.push((name.clone(), *range, help));
                    }
                    std::collections::hash_map::Entry::Vacant(e) => {
                        e.insert((&fd.rel_path, *line, *col));
                    }
                }
            }

            if !file_duplicates.is_empty() {
                result.insert(fd.rel_path.clone(), file_duplicates);
            }
        }
    }

    result
}
