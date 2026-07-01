use crate::helpers::{CliTest, CommandExt};

#[test]
fn test_s3_method_arguments_not_flagged() -> anyhow::Result<()> {
    // `print.foo` is registered in NAMESPACE as an S3 method. Its signature
    // is dictated by the `print` generic, so unused parameters like `...`
    // — and any other unused params — should not be flagged.
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        ("NAMESPACE", "S3method(print, foo)\n"),
        (
            "R/methods.R",
            "print.foo <- function(x, extra, ...) {\n  cat(\"foo\\n\")\n}\n",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_argument")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_regular_function_unused_arg_is_flagged() -> anyhow::Result<()> {
    // Same package layout, but a regular function — not an S3 method — has
    // its unused argument flagged.
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        ("NAMESPACE", ""),
        ("R/helpers.R", "f <- function(x, y) x\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_argument")
            .run()
            .normalize_os_executable_name(),
        @r"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_argument
     --> R/helpers.R:1:18
      |
    1 | f <- function(x, y) x
      |                  - Argument `y` is defined but never used.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_package_hook_arguments_not_flagged() -> anyhow::Result<()> {
    // `.onLoad` has a runtime-imposed signature (`libname, pkgname`); the
    // body often only uses one of them, but we must not flag the other.
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        ("NAMESPACE", ""),
        (
            "R/zzz.R",
            ".onLoad <- function(libname, pkgname) {\n  invisible()\n}\n",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_argument")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}
