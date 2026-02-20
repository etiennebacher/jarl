pub(crate) mod duplicated_function_definition;

#[cfg(test)]
mod tests {
    use super::duplicated_function_definition::*;
    use std::fs;
    use tempfile::TempDir;

    // ── scan_top_level_assignments ─────────────────────────────────────────

    #[test]
    fn test_scan_arrow_assignment() {
        let assignments = scan_top_level_assignments("foo <- function() 1\nbar <- function() 2\n");
        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments[0].0, "foo");
        assert_eq!(assignments[1].0, "bar");
    }

    #[test]
    fn test_scan_equals_assignment() {
        let assignments = scan_top_level_assignments("foo = function() 1\n");
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].0, "foo");
    }

    #[test]
    fn test_scan_ignores_non_function_assignments() {
        let assignments = scan_top_level_assignments("foo <- 1\nbar <- 'hello'\n");
        assert!(
            assignments.is_empty(),
            "non-function assignments should be ignored"
        );
    }

    #[test]
    fn test_scan_ignores_super_assignment() {
        let assignments = scan_top_level_assignments("foo <<- function() 1\n");
        assert!(assignments.is_empty(), "<<- should be ignored");
    }

    #[test]
    fn test_scan_ignores_right_assignment() {
        let assignments = scan_top_level_assignments("function() 1 -> foo\n");
        assert!(assignments.is_empty(), "right-assignment should be ignored");
    }

    #[test]
    fn test_scan_ignores_subscript_lhs() {
        // foo[1] and foo[[1]]: the `[` immediately after `foo` means no `<-` follows
        let assignments =
            scan_top_level_assignments("foo[1] <- function() 1\nfoo[[1]] <- function() 2\n");
        assert!(assignments.is_empty(), "subscript LHS should be ignored");
    }

    #[test]
    fn test_scan_ignores_indented_lines() {
        // Indented function assignments (inside a body) must not be collected.
        let assignments =
            scan_top_level_assignments("outer <- function() {\n  inner <- function() 1\n}\n");
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].0, "outer");
    }

    #[test]
    fn test_scan_lambda() {
        let assignments = scan_top_level_assignments("foo <- \\(x) x + 1\n");
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].0, "foo");
    }

    #[test]
    fn test_scan_function_keyword_prefix() {
        // `functionalities <- 1` should not be matched as a function assignment
        let assignments = scan_top_level_assignments("functionalities <- 1\n");
        assert!(assignments.is_empty());
    }

    // ── find_package_root ─────────────────────────────────────────────────

    #[test]
    fn test_find_package_root_basic() {
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();
        fs::write(dir.path().join("DESCRIPTION"), "Package: test").unwrap();
        let file = r_dir.join("foo.R");
        fs::write(&file, "").unwrap();

        let root = find_package_root(&file);
        assert_eq!(root, Some(dir.path().to_path_buf()));
    }

    #[test]
    fn test_find_package_root_no_description() {
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();
        let file = r_dir.join("foo.R");
        fs::write(&file, "").unwrap();

        let root = find_package_root(&file);
        assert!(root.is_none(), "no DESCRIPTION → no package root");
    }

    #[test]
    fn test_find_package_root_not_in_r_dir() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("DESCRIPTION"), "Package: test").unwrap();
        let file = dir.path().join("foo.R");
        fs::write(&file, "").unwrap();

        let root = find_package_root(&file);
        assert!(root.is_none(), "file not inside R/ → no package root");
    }

    // ── compute_package_duplicate_assignments ─────────────────────────────

    #[test]
    fn test_same_file_duplicates() {
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();
        fs::write(dir.path().join("DESCRIPTION"), "Package: test").unwrap();

        let file = r_dir.join("foo.R");
        fs::write(&file, "foo <- function() 1\nfoo <- function() 2\n").unwrap();

        let result = compute_package_duplicate_assignments(std::slice::from_ref(&file));

        // The second `foo` should be flagged, but the first should not.
        // The map has one entry for foo.R
        assert_eq!(result.len(), 1, "expected one file with duplicates");
        let (_, dupes) = result.iter().next().unwrap();
        assert_eq!(dupes.len(), 1);
        assert_eq!(dupes[0].0, "foo");
    }

    #[test]
    fn test_cross_file_duplicates() {
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();
        fs::write(dir.path().join("DESCRIPTION"), "Package: test").unwrap();

        // aaa.R comes alphabetically first → defines `foo` first (not flagged)
        let file_a = r_dir.join("aaa.R");
        fs::write(&file_a, "foo <- function() 1\n").unwrap();

        // bbb.R comes second → its `foo` is a duplicate
        let file_b = r_dir.join("bbb.R");
        fs::write(&file_b, "foo <- function() 2\n").unwrap();

        let result = compute_package_duplicate_assignments(&[file_a.clone(), file_b.clone()]);

        // Only bbb.R should have a diagnostic
        assert_eq!(result.len(), 1, "expected exactly one file with duplicates");
        let (flagged_path, dupes) = result.iter().next().unwrap();
        assert!(
            flagged_path.to_string_lossy().contains("bbb"),
            "bbb.R should be flagged, got: {flagged_path:?}"
        );
        assert_eq!(dupes.len(), 1);
        assert_eq!(dupes[0].0, "foo");
    }

    #[test]
    fn test_non_package_files_ignored() {
        let dir = TempDir::new().unwrap();
        // No DESCRIPTION file → not a package
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();

        let file = r_dir.join("foo.R");
        fs::write(&file, "foo <- function() 1\nfoo <- function() 2\n").unwrap();

        let result = compute_package_duplicate_assignments(std::slice::from_ref(&file));

        assert!(
            result.is_empty(),
            "non-package files should produce no results"
        );
    }

    #[test]
    fn test_unique_names_not_flagged() {
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();
        fs::write(dir.path().join("DESCRIPTION"), "Package: test").unwrap();

        let file_a = r_dir.join("a.R");
        fs::write(&file_a, "foo <- function() 1\n").unwrap();
        let file_b = r_dir.join("b.R");
        fs::write(&file_b, "bar <- function() 2\n").unwrap();

        let result = compute_package_duplicate_assignments(&[file_a.clone(), file_b.clone()]);

        assert!(result.is_empty(), "unique names should not be flagged");
    }
}
