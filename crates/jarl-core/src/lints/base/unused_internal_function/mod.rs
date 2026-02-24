pub(crate) mod unused_internal_function;

#[cfg(test)]
mod tests {
    use super::unused_internal_function::*;
    use std::fs;
    use tempfile::TempDir;

    // ── parse_namespace_exports ─────────────────────────────────────────

    #[test]
    fn test_parse_simple_exports() {
        let content = "export(foo)\nexport(bar)\n";
        let exports = parse_namespace_exports(content, &[]);
        assert!(exports.contains("foo"));
        assert!(exports.contains("bar"));
        assert_eq!(exports.len(), 2);
    }

    #[test]
    fn test_parse_multi_export() {
        let content = "export(foo, bar, baz)\n";
        let exports = parse_namespace_exports(content, &[]);
        assert!(exports.contains("foo"));
        assert!(exports.contains("bar"));
        assert!(exports.contains("baz"));
        assert_eq!(exports.len(), 3);
    }

    #[test]
    fn test_parse_export_pattern() {
        let content = "exportPattern(\"^[[:alpha:]]\")\n";
        let all_names = vec!["foo", "bar", ".hidden"];
        let exports = parse_namespace_exports(content, &all_names);
        assert!(exports.contains("foo"));
        assert!(exports.contains("bar"));
        assert!(!exports.contains(".hidden"));
    }

    #[test]
    fn test_parse_s3method() {
        let content = "S3method(print, myclass)\n";
        let exports = parse_namespace_exports(content, &[]);
        assert!(exports.contains("print.myclass"));
    }

    #[test]
    fn test_parse_comments_ignored() {
        let content = "# export(secret)\nexport(public)\n";
        let exports = parse_namespace_exports(content, &[]);
        assert!(!exports.contains("secret"));
        assert!(exports.contains("public"));
    }

    #[test]
    fn test_parse_s3method_namespaced_generic() {
        let content = "S3method(pkg::record_print, tinytable)\n";
        let exports = parse_namespace_exports(content, &[]);
        assert!(exports.contains("record_print.tinytable"));
    }

    #[test]
    fn test_parse_s3method_triple_colon() {
        let content = "S3method(pkg:::record_print, tinytable)\n";
        let exports = parse_namespace_exports(content, &[]);
        assert!(exports.contains("record_print.tinytable"));
    }

    #[test]
    fn test_parse_s3method_with_explicit_method() {
        let content = "S3method(print, myclass, print_myclass_impl)\n";
        let exports = parse_namespace_exports(content, &[]);
        assert!(exports.contains("print_myclass_impl"));
        assert!(!exports.contains("print.myclass"));
    }

    #[test]
    fn test_parse_s3method_with_if_guard() {
        let content = "if (getRversion() >= \"4.4.0\") S3method(sort_by, data.table)\n";
        let exports = parse_namespace_exports(content, &[]);
        assert!(
            exports.contains("sort_by.data.table"),
            "expected sort_by.data.table, got: {:?}",
            exports
        );
    }

    #[test]
    fn test_parse_export_with_if_guard() {
        let content = "if (getRversion() >= \"4.4.0\") export(sort_by)\n";
        let exports = parse_namespace_exports(content, &[]);
        assert!(exports.contains("sort_by"));
    }

    // ── scan_symbols ─────────────────────────────────────────────────────

    #[test]
    fn test_scan_symbols_from_calls() {
        let syms = scan_symbols("foo(1)\nbar(x, y)\n");
        assert!(syms.contains_key("foo"));
        assert!(syms.contains_key("bar"));
        assert!(syms.contains_key("x"));
        assert!(syms.contains_key("y"));
    }

    #[test]
    fn test_scan_symbols_from_assignments() {
        let syms = scan_symbols("x <- 1\ny + z\n");
        assert!(syms.contains_key("x"));
        assert!(syms.contains_key("y"));
        assert!(syms.contains_key("z"));
    }

    #[test]
    fn test_scan_symbols_nested() {
        let syms = scan_symbols("outer(inner(x))\n");
        assert!(syms.contains_key("outer"));
        assert!(syms.contains_key("inner"));
        assert!(syms.contains_key("x"));
    }

    #[test]
    fn test_scan_symbols_ignores_comments() {
        let syms = scan_symbols("# foo(bar)\nreal()\n");
        assert!(!syms.contains_key("foo"));
        assert!(!syms.contains_key("bar"));
        assert!(syms.contains_key("real"));
    }

    #[test]
    fn test_scan_symbols_ignores_indented_comments() {
        let syms = scan_symbols("  # foo(bar)\n");
        assert!(!syms.contains_key("foo"));
    }

    #[test]
    fn test_scan_symbols_ignores_numbers() {
        let syms = scan_symbols("123 + foo\n");
        assert!(syms.contains_key("foo"));
        assert!(!syms.contains_key("123"));
    }

    // ── compute_package_unused_internal_functions ────────────────────────

    #[test]
    fn test_unused_function_flagged() {
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();
        fs::write(dir.path().join("DESCRIPTION"), "Package: test").unwrap();
        fs::write(dir.path().join("NAMESPACE"), "export(public_fn)\n").unwrap();

        let file_a = r_dir.join("public.R");
        fs::write(&file_a, "public_fn <- function() 1\n").unwrap();

        let file_b = r_dir.join("unused.R");
        fs::write(&file_b, "unused_helper <- function() 2\n").unwrap();

        let result = compute_package_unused_internal_functions(&[file_a.clone(), file_b.clone()]);

        // unused_helper is not exported and never called → flagged
        let has_unused = result
            .values()
            .any(|v| v.iter().any(|(n, _, _)| n == "unused_helper"));
        assert!(has_unused, "unused_helper should be flagged");

        // public_fn is exported → not flagged
        let has_public = result
            .values()
            .any(|v| v.iter().any(|(n, _, _)| n == "public_fn"));
        assert!(!has_public, "public_fn should not be flagged (exported)");
    }

    #[test]
    fn test_called_internal_not_flagged() {
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();
        fs::write(dir.path().join("DESCRIPTION"), "Package: test").unwrap();
        fs::write(dir.path().join("NAMESPACE"), "export(public_fn)\n").unwrap();

        let file_a = r_dir.join("public.R");
        fs::write(&file_a, "public_fn <- function() helper()\n").unwrap();

        let file_b = r_dir.join("helper.R");
        fs::write(&file_b, "helper <- function() 1\n").unwrap();

        let result = compute_package_unused_internal_functions(&[file_a.clone(), file_b.clone()]);

        assert!(
            result.is_empty(),
            "helper is called by public_fn, should not be flagged"
        );
    }

    #[test]
    fn test_internal_s3_method_not_flagged() {
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();
        fs::write(dir.path().join("DESCRIPTION"), "Package: test").unwrap();
        fs::write(dir.path().join("NAMESPACE"), "export(public_fn)\n").unwrap();

        // print.myclass is an internal S3 method; print() is called elsewhere
        let file_a = r_dir.join("public.R");
        fs::write(&file_a, "public_fn <- function(x) print(x)\n").unwrap();

        let file_b = r_dir.join("methods.R");
        fs::write(&file_b, "print.myclass <- function(x, ...) cat(x)\n").unwrap();

        let result = compute_package_unused_internal_functions(&[file_a.clone(), file_b.clone()]);

        assert!(
            result.is_empty(),
            "print.myclass is a probable S3 method, should not be flagged"
        );
    }

    #[test]
    fn test_internal_s3_method_dotted_class_not_flagged() {
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();
        fs::write(dir.path().join("DESCRIPTION"), "Package: test").unwrap();
        fs::write(dir.path().join("NAMESPACE"), "export(public_fn)\n").unwrap();

        // sort_by.data.table — generic is `sort_by`, class is `data.table`
        let file_a = r_dir.join("public.R");
        fs::write(&file_a, "public_fn <- function(x) sort_by(x)\n").unwrap();

        let file_b = r_dir.join("methods.R");
        fs::write(&file_b, "sort_by.data.table <- function(x, ...) x\n").unwrap();

        let result = compute_package_unused_internal_functions(&[file_a.clone(), file_b.clone()]);

        assert!(
            result.is_empty(),
            "sort_by.data.table is a probable S3 method, should not be flagged"
        );
    }

    #[test]
    fn test_same_file_usage_not_flagged() {
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();
        fs::write(dir.path().join("DESCRIPTION"), "Package: test").unwrap();
        fs::write(dir.path().join("NAMESPACE"), "export(public_fn)\n").unwrap();

        // helper is defined and called in the same file
        let file = r_dir.join("main.R");
        fs::write(
            &file,
            "public_fn <- function() helper()\nhelper <- function() 1\n",
        )
        .unwrap();

        let result = compute_package_unused_internal_functions(std::slice::from_ref(&file));

        assert!(
            result.is_empty(),
            "helper is called in the same file, should not be flagged"
        );
    }

    #[test]
    fn test_no_namespace_skips_package() {
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();
        fs::write(dir.path().join("DESCRIPTION"), "Package: test").unwrap();
        // No NAMESPACE file

        let file = r_dir.join("foo.R");
        fs::write(&file, "foo <- function() 1\n").unwrap();

        let result = compute_package_unused_internal_functions(std::slice::from_ref(&file));

        assert!(
            result.is_empty(),
            "without NAMESPACE, package should be skipped"
        );
    }

    #[test]
    fn test_export_pattern_not_flagged() {
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();
        fs::write(dir.path().join("DESCRIPTION"), "Package: test").unwrap();
        fs::write(
            dir.path().join("NAMESPACE"),
            "exportPattern(\"^[[:alpha:]]\")\n",
        )
        .unwrap();

        let file = r_dir.join("foo.R");
        fs::write(&file, "foo <- function() 1\n").unwrap();

        let result = compute_package_unused_internal_functions(std::slice::from_ref(&file));

        assert!(
            result.is_empty(),
            "foo matches exportPattern, should not be flagged"
        );
    }

    #[test]
    fn test_function_used_in_tests_not_flagged() {
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();
        let tests_dir = dir.path().join("tests").join("testthat");
        fs::create_dir_all(&tests_dir).unwrap();
        fs::write(dir.path().join("DESCRIPTION"), "Package: test").unwrap();
        fs::write(dir.path().join("NAMESPACE"), "export(public_fn)\n").unwrap();

        let file_a = r_dir.join("public.R");
        fs::write(&file_a, "public_fn <- function() 1\n").unwrap();

        let file_b = r_dir.join("internal.R");
        fs::write(&file_b, "internal_helper <- function() 2\n").unwrap();

        // internal_helper is used only in a test file
        let test_file = tests_dir.join("test-internal.R");
        fs::write(&test_file, "test_that('works', { internal_helper() })\n").unwrap();

        let result = compute_package_unused_internal_functions(&[file_a.clone(), file_b.clone()]);

        let has_internal = result
            .values()
            .any(|v| v.iter().any(|(n, _, _)| n == "internal_helper"));
        assert!(
            !has_internal,
            "internal_helper is used in tests/, should not be flagged"
        );
    }

    #[test]
    fn test_function_used_in_inst_not_flagged() {
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();
        let inst_dir = dir.path().join("inst").join("tinytest");
        fs::create_dir_all(&inst_dir).unwrap();
        fs::write(dir.path().join("DESCRIPTION"), "Package: test").unwrap();
        fs::write(dir.path().join("NAMESPACE"), "export(public_fn)\n").unwrap();

        let file_a = r_dir.join("public.R");
        fs::write(&file_a, "public_fn <- function() 1\n").unwrap();

        let file_b = r_dir.join("internal.R");
        fs::write(&file_b, "inst_helper <- function() 2\n").unwrap();

        // inst_helper is used only in an inst/ file
        let inst_file = inst_dir.join("test_inst.R");
        fs::write(&inst_file, "expect_equal(inst_helper(), 2)\n").unwrap();

        let result = compute_package_unused_internal_functions(&[file_a.clone(), file_b.clone()]);

        let has_inst = result
            .values()
            .any(|v| v.iter().any(|(n, _, _)| n == "inst_helper"));
        assert!(
            !has_inst,
            "inst_helper is used in inst/tinytest, should not be flagged"
        );
    }

    #[test]
    fn test_function_used_in_inst_not_tinytest_flagged() {
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();
        let inst_dir = dir.path().join("inst").join("foobar");
        fs::create_dir_all(&inst_dir).unwrap();
        fs::write(dir.path().join("DESCRIPTION"), "Package: test").unwrap();
        fs::write(dir.path().join("NAMESPACE"), "export(public_fn)\n").unwrap();

        let file_a = r_dir.join("public.R");
        fs::write(&file_a, "public_fn <- function() 1\n").unwrap();

        let file_b = r_dir.join("internal.R");
        fs::write(&file_b, "inst_helper <- function() 2\n").unwrap();

        // inst_helper is used only in an inst/ file
        let inst_file = inst_dir.join("test_inst.R");
        fs::write(&inst_file, "expect_equal(inst_helper(), 2)\n").unwrap();

        let result = compute_package_unused_internal_functions(&[file_a.clone(), file_b.clone()]);

        let has_inst = result
            .values()
            .any(|v| v.iter().any(|(n, _, _)| n == "inst_helper"));
        assert!(
            has_inst,
            "inst_helper is used in inst/ but not in inst/tinytest, should be flagged"
        );
    }

    #[test]
    fn test_non_package_files_ignored() {
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();
        // No DESCRIPTION → not a package

        let file = r_dir.join("foo.R");
        fs::write(&file, "foo <- function() 1\n").unwrap();

        let result = compute_package_unused_internal_functions(std::slice::from_ref(&file));

        assert!(
            result.is_empty(),
            "non-package files should produce no results"
        );
    }
}
