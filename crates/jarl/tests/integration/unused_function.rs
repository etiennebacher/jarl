use crate::helpers::{CliTest, CommandExt};

/// Run `jarl check . --select unused_function` in `case`.
fn check(case: &CliTest) -> String {
    case.command()
        .arg("check")
        .arg(".")
        .arg("--select")
        .arg("unused_function")
        .run()
        .normalize_os_executable_name()
        .to_string()
}

#[test]
fn test_unused_function_flagged() -> anyhow::Result<()> {
    let case = CliTest::package_with_files([
        ("NAMESPACE", "export(public_fn)\n"),
        ("R/public.R", "public_fn <- function() 1\n"),
        ("R/unused.R", "unused_helper <- function() 2\n"),
    ])?;

    insta::assert_snapshot!(check(&case), @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_function
     --> R/unused.R:1:1
      |
    1 | unused_helper <- function() 2
      | ------------- `unused_helper` is defined but is never called in this package nor is it exported in NAMESPACE.
      |
      = help: Defined at R/unused.R:1:1.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    ");
    Ok(())
}

#[test]
fn test_called_internal_not_flagged() -> anyhow::Result<()> {
    let case = CliTest::package_with_files([
        ("NAMESPACE", "export(public_fn)\n"),
        ("R/public.R", "public_fn <- function() helper()\n"),
        ("R/helper.R", "helper <- function() 1\n"),
    ])?;

    insta::assert_snapshot!(check(&case), @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    ");
    Ok(())
}

#[test]
fn test_non_ascii_function_used_across_files() -> anyhow::Result<()> {
    let case = CliTest::package_with_files([
        ("NAMESPACE", "export(public_fn)\n"),
        (
            "R/definitions.R",
            "中文函数 <- function(x) x\n未使用 <- function() 1\nunused_helper <- function() 2\n",
        ),
        ("R/uses.R", "use_functions <- function(x) 中文函数(x)\n"),
    ])?;

    insta::assert_snapshot!(check(&case), @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_function
     --> R/definitions.R:2:1
      |
    2 | 未使用 <- function() 1
      | ------ `未使用` is defined but is never called in this package nor is it exported in NAMESPACE.
      |
      = help: Defined at R/definitions.R:2:1.

    warning: unused_function
     --> R/definitions.R:3:1
      |
    3 | unused_helper <- function() 2
      | ------------- `unused_helper` is defined but is never called in this package nor is it exported in NAMESPACE.
      |
      = help: Defined at R/definitions.R:3:1.

    warning: unused_function
     --> R/uses.R:1:1
      |
    1 | use_functions <- function(x) 中文函数(x)
      | ------------- `use_functions` is defined but is never called in this package nor is it exported in NAMESPACE.
      |
      = help: Defined at R/uses.R:1:1.


    ── Summary ──────────────────────────────────────
    Found 3 errors.

    ----- stderr -----
    ");
    Ok(())
}

#[test]
fn test_internal_s3_method_not_flagged() -> anyhow::Result<()> {
    let case = CliTest::package_with_files([
        ("NAMESPACE", "export(public_fn)\n"),
        ("R/public.R", "public_fn <- function(x) print(x)\n"),
        ("R/methods.R", "print.myclass <- function(x, ...) cat(x)\n"),
    ])?;

    insta::assert_snapshot!(check(&case), @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    ");
    Ok(())
}

#[test]
fn test_internal_s3_method_dotted_class_not_flagged() -> anyhow::Result<()> {
    let case = CliTest::package_with_files([
        ("NAMESPACE", "export(public_fn)\n"),
        ("R/public.R", "public_fn <- function(x) sort_by(x)\n"),
        ("R/methods.R", "sort_by.data.table <- function(x, ...) x\n"),
    ])?;

    insta::assert_snapshot!(check(&case), @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    ");
    Ok(())
}

#[test]
fn test_same_file_usage_not_flagged() -> anyhow::Result<()> {
    let case = CliTest::package_with_files([
        ("NAMESPACE", "export(public_fn)\n"),
        (
            "R/main.R",
            "public_fn <- function() helper()\nhelper <- function() 1\n",
        ),
    ])?;

    insta::assert_snapshot!(check(&case), @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    ");
    Ok(())
}

#[test]
fn test_no_namespace_skips_package() -> anyhow::Result<()> {
    let case = CliTest::package_with_files([("R/foo.R", "foo <- function() 1\n")])?;

    insta::assert_snapshot!(check(&case), @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    ");
    Ok(())
}

#[test]
fn test_export_pattern_not_flagged() -> anyhow::Result<()> {
    let case = CliTest::package_with_files([
        ("NAMESPACE", "exportPattern(\"^[[:alpha:]]\")\n"),
        ("R/foo.R", "foo <- function() 1\n"),
    ])?;

    insta::assert_snapshot!(check(&case), @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    ");
    Ok(())
}

#[test]
fn test_non_package_files_ignored() -> anyhow::Result<()> {
    // No DESCRIPTION: not a package.
    let case = CliTest::with_files([("R/foo.R", "foo <- function() 1\n")])?;

    insta::assert_snapshot!(check(&case), @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    ");
    Ok(())
}

// ── Usage in tests/, inst/ and src/ ─────────────────────────────────

#[test]
fn test_function_used_in_tests_not_flagged() -> anyhow::Result<()> {
    let case = CliTest::package_with_files([
        ("NAMESPACE", "export(public_fn)\n"),
        ("R/public.R", "public_fn <- function() 1\n"),
        ("R/internal.R", "internal_helper <- function() 2\n"),
        (
            "tests/testthat/test-internal.R",
            "test_that('works', { internal_helper() })\n",
        ),
    ])?;

    insta::assert_snapshot!(check(&case), @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    ");
    Ok(())
}

#[test]
fn test_function_used_in_inst_tinytest_not_flagged() -> anyhow::Result<()> {
    let case = CliTest::package_with_files([
        ("NAMESPACE", "export(public_fn)\n"),
        ("R/public.R", "public_fn <- function() 1\n"),
        ("R/internal.R", "inst_helper <- function() 2\n"),
        (
            "inst/tinytest/test_inst.R",
            "expect_equal(inst_helper(), 2)\n",
        ),
    ])?;

    insta::assert_snapshot!(check(&case), @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    ");
    Ok(())
}

#[test]
fn test_function_used_in_inst_other_dir_flagged() -> anyhow::Result<()> {
    let case = CliTest::package_with_files([
        ("NAMESPACE", "export(public_fn)\n"),
        ("R/public.R", "public_fn <- function() 1\n"),
        ("R/internal.R", "inst_helper <- function() 2\n"),
        (
            "inst/foobar/test_inst.R",
            "expect_equal(inst_helper(), 2)\n",
        ),
    ])?;

    insta::assert_snapshot!(check(&case), @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_function
     --> R/internal.R:1:1
      |
    1 | inst_helper <- function() 2
      | ----------- `inst_helper` is defined but is never called in this package nor is it exported in NAMESPACE.
      |
      = help: Defined at R/internal.R:1:1.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    ");
    Ok(())
}

#[test]
fn test_function_used_in_src_cpp_not_flagged() -> anyhow::Result<()> {
    let case = CliTest::package_with_files([
        ("NAMESPACE", "export(public_fn)\n"),
        ("R/public.R", "public_fn <- function() 1\n"),
        ("R/internal.R", "dplyr_internal_signal <- function() 2\n"),
        (
            "src/init.cpp",
            "SEXP symbols::dplyr_internal_signal = Rf_install(\"dplyr_internal_signal\");\n",
        ),
    ])?;

    insta::assert_snapshot!(check(&case), @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    ");
    Ok(())
}

// ── Unused functions defined in tests/ and inst/ ────────────────────

#[test]
fn test_unused_function_in_tests_flagged() -> anyhow::Result<()> {
    let case = CliTest::package_with_files([
        ("NAMESPACE", "export(public_fn)\n"),
        ("R/public.R", "public_fn <- function() 1\n"),
        (
            "tests/testthat/helper.R",
            "unused_test_helper <- function() 42\n",
        ),
    ])?;

    insta::assert_snapshot!(check(&case), @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_function
     --> tests/testthat/helper.R:1:1
      |
    1 | unused_test_helper <- function() 42
      | ------------------ `unused_test_helper` is defined but is never called in this package nor is it exported in NAMESPACE.
      |
      = help: Defined at tests/testthat/helper.R:1:1 but never called in tests/


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    ");
    Ok(())
}

#[test]
fn test_used_function_in_tests_not_flagged() -> anyhow::Result<()> {
    let case = CliTest::package_with_files([
        ("NAMESPACE", "export(public_fn)\n"),
        ("R/public.R", "public_fn <- function() 1\n"),
        ("tests/testthat/helper.R", "test_helper <- function() 42\n"),
        (
            "tests/testthat/test-foo.R",
            "test_that('works', { test_helper() })\n",
        ),
    ])?;

    insta::assert_snapshot!(check(&case), @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    ");
    Ok(())
}

#[test]
fn test_unused_function_in_inst_tinytest_flagged() -> anyhow::Result<()> {
    let case = CliTest::package_with_files([
        ("NAMESPACE", "export(public_fn)\n"),
        ("R/public.R", "public_fn <- function() 1\n"),
        (
            "inst/tinytest/helper.R",
            "unused_inst_helper <- function() 42\n",
        ),
    ])?;

    insta::assert_snapshot!(check(&case), @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_function
     --> inst/tinytest/helper.R:1:1
      |
    1 | unused_inst_helper <- function() 42
      | ------------------ `unused_inst_helper` is defined but is never called in this package nor is it exported in NAMESPACE.
      |
      = help: Defined at inst/tinytest/helper.R:1:1 but never called in inst/tinytest/


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    ");
    Ok(())
}

#[test]
fn test_unused_function_in_inst_tests_flagged() -> anyhow::Result<()> {
    let case = CliTest::package_with_files([
        ("NAMESPACE", "export(public_fn)\n"),
        ("R/public.R", "public_fn <- function() 1\n"),
        (
            "inst/tests/helper.R",
            "unused_inst_helper <- function() 42\n",
        ),
    ])?;

    insta::assert_snapshot!(check(&case), @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_function
     --> inst/tests/helper.R:1:1
      |
    1 | unused_inst_helper <- function() 42
      | ------------------ `unused_inst_helper` is defined but is never called in this package nor is it exported in NAMESPACE.
      |
      = help: Defined at inst/tests/helper.R:1:1 but never called in inst/tests/


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    ");
    Ok(())
}

#[test]
fn test_cross_scope_independence() -> anyhow::Result<()> {
    // A use in R/ doesn't save a definition that lives in tests/.
    let case = CliTest::package_with_files([
        ("NAMESPACE", "export(public_fn)\n"),
        ("R/public.R", "public_fn <- function() test_only_helper()\n"),
        (
            "tests/testthat/helper.R",
            "test_only_helper <- function() 42\n",
        ),
    ])?;

    insta::assert_snapshot!(check(&case), @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_function
     --> tests/testthat/helper.R:1:1
      |
    1 | test_only_helper <- function() 42
      | ---------------- `test_only_helper` is defined but is never called in this package nor is it exported in NAMESPACE.
      |
      = help: Defined at tests/testthat/helper.R:1:1 but never called in tests/


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    ");
    Ok(())
}

// ── threshold-ignore ────────────────────────────────────────────────

fn package_with_n_unused(n: usize, config: &str) -> anyhow::Result<CliTest> {
    let case = CliTest::package_with_files([
        ("NAMESPACE", "export(public_fn)\n"),
        ("R/public.R", "public_fn <- function() 1\n"),
        ("jarl.toml", config),
    ])?;
    for i in 1..=n {
        case.write_file(
            format!("R/unused_{i}.R"),
            &format!("unused_fn_{i} <- function() {i}\n"),
        )?;
    }
    Ok(case)
}

fn check_default_rules(case: &CliTest) -> String {
    case.command()
        .arg("check")
        .arg(".")
        .run()
        .normalize_os_executable_name()
        .to_string()
}

#[test]
fn test_threshold_exceeded_hides_diagnostics() -> anyhow::Result<()> {
    let case = package_with_n_unused(5, "[lint.unused_function]\nthreshold-ignore = 3\n")?;

    insta::assert_snapshot!(check_default_rules(&case), @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ── Warnings ─────────────────────────────────────
    5 `unused_function` diagnostics hidden (likely false positives).
    To show them:
      - set 'threshold-ignore' in `[lint.unused_function]` in jarl.toml,
      - or explicitly include 'unused_function' in the set of rules.

    ----- stderr -----
    ");
    Ok(())
}

#[test]
fn test_threshold_not_exceeded_shows_diagnostics() -> anyhow::Result<()> {
    let case = package_with_n_unused(2, "[lint.unused_function]\nthreshold-ignore = 3\n")?;

    insta::assert_snapshot!(check_default_rules(&case), @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_function
     --> R/unused_1.R:1:1
      |
    1 | unused_fn_1 <- function() 1
      | ----------- `unused_fn_1` is defined but is never called in this package nor is it exported in NAMESPACE.
      |
      = help: Defined at R/unused_1.R:1:1.

    warning: unused_function
     --> R/unused_2.R:1:1
      |
    1 | unused_fn_2 <- function() 2
      | ----------- `unused_fn_2` is defined but is never called in this package nor is it exported in NAMESPACE.
      |
      = help: Defined at R/unused_2.R:1:1.


    ── Summary ──────────────────────────────────────
    Found 2 errors.

    ----- stderr -----
    ");
    Ok(())
}
