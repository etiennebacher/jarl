pub(crate) mod unused_function;

#[cfg(test)]
mod tests {
    use super::unused_function::*;
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
    fn test_scan_symbols_includes_roxygen_comments() {
        let syms = scan_symbols("#' \\Sexpr[stage=render]{dplyr:::methods_rd(\"rows_insert\")}\n");
        assert!(syms.contains_key("methods_rd"));
    }

    #[test]
    fn test_scan_symbols_ignores_numbers() {
        let syms = scan_symbols("123 + foo\n");
        assert!(syms.contains_key("foo"));
        assert!(!syms.contains_key("123"));
    }

    // ── compute_package_unused_functions ────────────────────────

    fn default_options() -> ResolvedUnusedFunctionOptions {
        ResolvedUnusedFunctionOptions::resolve(None).unwrap()
    }

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

        let result =
            compute_package_unused_functions(&[file_a.clone(), file_b.clone()], &default_options());

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

        let result =
            compute_package_unused_functions(&[file_a.clone(), file_b.clone()], &default_options());

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

        let result =
            compute_package_unused_functions(&[file_a.clone(), file_b.clone()], &default_options());

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

        let result =
            compute_package_unused_functions(&[file_a.clone(), file_b.clone()], &default_options());

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

        let result =
            compute_package_unused_functions(std::slice::from_ref(&file), &default_options());

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

        let result =
            compute_package_unused_functions(std::slice::from_ref(&file), &default_options());

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

        let result =
            compute_package_unused_functions(std::slice::from_ref(&file), &default_options());

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

        let result =
            compute_package_unused_functions(&[file_a.clone(), file_b.clone()], &default_options());

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

        let result =
            compute_package_unused_functions(&[file_a.clone(), file_b.clone()], &default_options());

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

        let result =
            compute_package_unused_functions(&[file_a.clone(), file_b.clone()], &default_options());

        let has_inst = result
            .values()
            .any(|v| v.iter().any(|(n, _, _)| n == "inst_helper"));
        assert!(
            has_inst,
            "inst_helper is used in inst/ but not in inst/tinytest, should be flagged"
        );
    }

    #[test]
    fn test_function_used_in_src_cpp_not_flagged() {
        let dir = TempDir::new().unwrap();
        let r_dir = dir.path().join("R");
        fs::create_dir(&r_dir).unwrap();
        let src_dir = dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();
        fs::write(dir.path().join("DESCRIPTION"), "Package: test").unwrap();
        fs::write(dir.path().join("NAMESPACE"), "export(public_fn)\n").unwrap();

        let file_a = r_dir.join("public.R");
        fs::write(&file_a, "public_fn <- function() 1\n").unwrap();

        let file_b = r_dir.join("internal.R");
        fs::write(&file_b, "dplyr_internal_signal <- function() 2\n").unwrap();

        // dplyr_internal_signal is referenced in a .cpp file
        let cpp_file = src_dir.join("init.cpp");
        fs::write(
            &cpp_file,
            "SEXP symbols::dplyr_internal_signal = Rf_install(\"dplyr_internal_signal\");\n",
        )
        .unwrap();

        let result =
            compute_package_unused_functions(&[file_a.clone(), file_b.clone()], &default_options());

        let has_signal = result
            .values()
            .any(|v| v.iter().any(|(n, _, _)| n == "dplyr_internal_signal"));
        assert!(
            !has_signal,
            "dplyr_internal_signal is used in src/*.cpp, should not be flagged"
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

        let result =
            compute_package_unused_functions(std::slice::from_ref(&file), &default_options());

        assert!(
            result.is_empty(),
            "non-package files should produce no results"
        );
    }

    // ── ResolvedUnusedFunctionOptions ────────────────────────────────────

    use crate::rule_options::unused_function::{
        ResolvedUnusedFunctionOptions, UnusedFunctionOptions,
    };

    #[test]
    fn test_threshold_ignore_default_is_50() {
        let resolved = ResolvedUnusedFunctionOptions::resolve(None).unwrap();
        assert_eq!(resolved.threshold_ignore, 50);
    }

    #[test]
    fn test_threshold_ignore_custom_value() {
        let opts = UnusedFunctionOptions { threshold_ignore: Some(10), ..Default::default() };
        let resolved = ResolvedUnusedFunctionOptions::resolve(Some(&opts)).unwrap();
        assert_eq!(resolved.threshold_ignore, 10);
    }

    #[test]
    fn test_threshold_ignore_none_uses_default() {
        let opts = UnusedFunctionOptions { threshold_ignore: None, ..Default::default() };
        let resolved = ResolvedUnusedFunctionOptions::resolve(Some(&opts)).unwrap();
        assert_eq!(resolved.threshold_ignore, 50);
    }

    #[test]
    fn test_threshold_ignore_zero() {
        let opts = UnusedFunctionOptions { threshold_ignore: Some(0), ..Default::default() };
        let resolved = ResolvedUnusedFunctionOptions::resolve(Some(&opts)).unwrap();
        assert_eq!(resolved.threshold_ignore, 0);
    }

    // ── skipped-functions ──────────────────────────────────────────────

    #[test]
    fn test_skipped_functions_invalid_regex() {
        let opts = UnusedFunctionOptions {
            skipped_functions: Some(vec!["^pl__".to_string(), "[invalid".to_string()]),
            ..Default::default()
        };
        let err = ResolvedUnusedFunctionOptions::resolve(Some(&opts)).unwrap_err();
        assert!(
            err.to_string().contains("[invalid"),
            "error should mention the bad pattern: {err}"
        );
    }

    // ── threshold-ignore end-to-end ─────────────────────────────────────

    use annotate_snippets::Renderer;
    use insta::assert_snapshot;

    use crate::check::check;
    use crate::config::{ArgsConfig, build_config};
    use crate::diagnostic::render_diagnostic;

    /// Create a minimal R package with `n` unused internal functions, run
    /// the linter, and return the rendered diagnostics as a string (same
    /// format the CLI emits).  Absolute temp paths are replaced with
    /// `[PKG]` for snapshot stability.
    fn check_package_with_n_unused(dir: &std::path::Path, n: usize) -> String {
        let r_dir = dir.join("R");
        fs::create_dir_all(&r_dir).unwrap();
        fs::write(
            dir.join("DESCRIPTION"),
            "Package: testpkg\nVersion: 0.1.0\n",
        )
        .unwrap();
        fs::write(dir.join("NAMESPACE"), "export(public_fn)\n").unwrap();
        fs::write(r_dir.join("public.R"), "public_fn <- function() 1\n").unwrap();

        let mut paths = vec![r_dir.join("public.R")];
        for i in 1..=n {
            let file = r_dir.join(format!("unused_{i}.R"));
            fs::write(&file, format!("unused_fn_{i} <- function() {i}\n")).unwrap();
            paths.push(file);
        }

        let args = ArgsConfig {
            files: paths.iter().map(|p| p.to_path_buf()).collect(),
            fix: false,
            unsafe_fixes: false,
            fix_only: false,
            select: "unused_function".to_string(),
            extend_select: String::new(),
            ignore: String::new(),
            min_r_version: None,
            allow_dirty: false,
            allow_no_vcs: true,
            assignment: None,
        };

        let config = build_config(&args, None, paths).unwrap();
        let results = check(config);

        let renderer = Renderer::plain();
        let mut all_diagnostics = Vec::new();

        for (path, result) in &results {
            if let Ok(diagnostics) = result {
                for d in diagnostics {
                    let content = fs::read_to_string(&d.filename).unwrap();
                    let rendered = render_diagnostic(&content, path, &d.message.name, d, &renderer);
                    all_diagnostics.push((d, rendered));
                }
            }
        }

        all_diagnostics.sort_by(|(a, _), (b, _)| a.cmp(b));

        let mut output = String::new();
        for (_, rendered) in &all_diagnostics {
            output.push_str(&format!("{rendered}\n"));
        }

        let count = all_diagnostics.len();
        if count > 0 {
            output.push_str(&format!(
                "Found {count} error{}.",
                if count == 1 { "" } else { "s" }
            ));
        }

        // Normalize temp directory paths for stable snapshots
        let dir_str = dir.to_string_lossy();
        output.replace(&*dir_str, "[PKG]").replace('\\', "/")
    }

    /// Simulate the threshold-ignore filtering that the CLI applies:
    /// if the number of `unused_function` diagnostics exceeds
    /// `threshold`, return a note; otherwise return the diagnostics.
    fn apply_threshold(diagnostics_output: &str, count: usize, threshold: usize) -> String {
        if count > threshold {
            format!(
                "All checks passed!\n\
                 Warning: {count} `unused_function` diagnostics hidden \
                 (likely false positives).\n\
                 To show them:\n  \
                 - set 'threshold-ignore' in `[lint.unused-function]` in jarl.toml,\n  \
                 - or explicitly include 'unused_function' in the set of rules."
            )
        } else {
            diagnostics_output.to_string()
        }
    }

    #[test]
    fn test_threshold_exceeded_hides_diagnostics() {
        let dir = TempDir::new().unwrap();
        let diagnostics_output = check_package_with_n_unused(dir.path(), 5);
        let count = 5;
        let threshold = 3;

        assert_snapshot!(
            apply_threshold(&diagnostics_output, count, threshold),
            @r"
        All checks passed!
        Warning: 5 `unused_function` diagnostics hidden (likely false positives).
        To show them:
          - set 'threshold-ignore' in `[lint.unused-function]` in jarl.toml,
          - or explicitly include 'unused_function' in the set of rules.
        "
        );
    }

    #[test]
    fn test_threshold_not_exceeded_shows_diagnostics() {
        let dir = TempDir::new().unwrap();
        let diagnostics_output = check_package_with_n_unused(dir.path(), 2);
        let count = 2;
        let threshold = 3;

        assert_snapshot!(
            apply_threshold(&diagnostics_output, count, threshold),
            @r"
        warning: unused_function
         --> [PKG]/R/unused_1.R:1:1
          |
        1 | unused_fn_1 <- function() 1
          | ----------- `unused_fn_1` is defined but never called in this package.
          |
          = help: Defined at [PKG]/R/unused_1.R:1:1 but never called
        warning: unused_function
         --> [PKG]/R/unused_2.R:1:1
          |
        1 | unused_fn_2 <- function() 2
          | ----------- `unused_fn_2` is defined but never called in this package.
          |
          = help: Defined at [PKG]/R/unused_2.R:1:1 but never called
        Found 2 errors.
        "
        );
    }
}
