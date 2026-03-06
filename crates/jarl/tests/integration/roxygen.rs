use std::process::Command;

use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

// ---------------------------------------------------------------------------
// Basic lint detection
// ---------------------------------------------------------------------------

#[test]
fn test_roxygen_examples_lint() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("test.R"),
        "\
#' Title
#' @param x A value
#' @examples
#' any(is.na(x))
foo <- function(x) x
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @r"
    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> test.R:4:4
      |
    4 | #' any(is.na(x))
      |    ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_roxygen_examples_if_lint() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("test.R"),
        "\
#' Title
#' @examplesIf interactive()
#' any(is.na(x))
foo <- function(x) x
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @r"
    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> test.R:3:4
      |
    3 | #' any(is.na(x))
      |    ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Clean examples produce no diagnostics
// ---------------------------------------------------------------------------

#[test]
fn test_roxygen_clean_examples() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("test.R"),
        "\
#' Title
#' @examples
#' x <- 1
foo <- function(x) x
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @r"
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

// ---------------------------------------------------------------------------
// Parse errors silently skipped
// ---------------------------------------------------------------------------

#[test]
fn test_roxygen_parse_error_skipped() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("test.R"),
        "\
#' Title
#' @examples
#' 1 +
foo <- function(x) x
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @r"
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

// ---------------------------------------------------------------------------
// Multiple roxygen blocks
// ---------------------------------------------------------------------------

#[test]
fn test_roxygen_multiple_blocks() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("test.R"),
        "\
#' First function
#' @examples
#' any(is.na(x))
foo <- function(x) x

#' Second function
#' @examples
#' any(is.na(y))
bar <- function(y) y
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @r"
    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> test.R:3:4
      |
    3 | #' any(is.na(x))
      |    ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_is_na
     --> test.R:8:4
      |
    8 | #' any(is.na(y))
      |    ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.

    ----- stderr -----
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// check-roxygen = false disables roxygen linting
// ---------------------------------------------------------------------------

#[test]
fn test_roxygen_disabled_via_toml() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("test.R"),
        "\
#' Title
#' @examples
#' any(is.na(x))
foo <- function(x) x
",
    )?;

    std::fs::write(
        directory.join("jarl.toml"),
        "\
[lint]
check-roxygen = false
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @r"
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

// ---------------------------------------------------------------------------
// \dontrun{} and \donttest{} — entire examples section skipped on parse error
// ---------------------------------------------------------------------------

/// When `\dontrun{}` or `\donttest{}` is present, the extracted code is not
/// valid R, so the entire examples section is silently skipped.
#[test]
fn test_roxygen_dontrun_skipped() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("test.R"),
        "\
#' Title
#' @examples
#' \\dontrun{
#' any(is.na(x))
#' }
foo <- function(x) x
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @r"
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
fn test_roxygen_donttest_skipped() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("test.R"),
        "\
#' Title
#' @examples
#' \\donttest{
#' any(is.na(x))
#' }
foo <- function(x) x
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @r"
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

/// Code before `\dontrun{}` in the same examples section is also skipped
/// because the whole extracted block fails to parse.
#[test]
fn test_roxygen_dontrun_skips_entire_section() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("test.R"),
        "\
#' Title
#' @examples
#' any(is.na(x))
#' \\dontrun{
#' y <- 1
#' }
foo <- function(x) x
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @r"
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

/// When one roxygen block has `\dontrun{}` and another doesn't, only the
/// clean block is linted.
#[test]
fn test_roxygen_dontrun_does_not_affect_other_blocks() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("test.R"),
        "\
#' First function
#' @examples
#' any(is.na(x))
foo <- function(x) x

#' Second function
#' @examples
#' any(is.na(y))
#' \\dontrun{
#' z <- 1
#' }
bar <- function(y) y
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @r"
    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> test.R:3:4
      |
    3 | #' any(is.na(x))
      |    ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// ##' is also a valid roxygen comment
// ---------------------------------------------------------------------------

#[test]
fn test_double_hash_is_roxygen() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("test.R"),
        "\
##' Title
##' @examples
##' any(is.na(x))
foo <- function(x) x
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @r"
    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> test.R:3:5
      |
    3 | ##' any(is.na(x))
      |     ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}
