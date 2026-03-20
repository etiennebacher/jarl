use std::process::Command;

use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

/// Set up a minimal R package directory structure inside the given directory.
/// Creates `DESCRIPTION` and `R/` subdirectory. Returns the path to `R/`.
fn setup_r_package(directory: &std::path::Path) -> std::path::PathBuf {
    std::fs::write(
        directory.join("DESCRIPTION"),
        "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
    )
    .unwrap();
    let r_dir = directory.join("R");
    std::fs::create_dir_all(&r_dir).unwrap();
    r_dir
}

// ---------------------------------------------------------------------------
// Basic lint detection
// ---------------------------------------------------------------------------

#[test]
fn test_roxygen_examples_lint() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();
    let r_dir = setup_r_package(directory);

    std::fs::write(
        r_dir.join("test.R"),
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
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/test.R:4:4
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
    let r_dir = setup_r_package(directory);

    std::fs::write(
        r_dir.join("test.R"),
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
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/test.R:3:4
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
    let r_dir = setup_r_package(directory);

    std::fs::write(
        r_dir.join("test.R"),
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

// ---------------------------------------------------------------------------
// Parse errors silently skipped
// ---------------------------------------------------------------------------

#[test]
fn test_roxygen_parse_error_skipped() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();
    let r_dir = setup_r_package(directory);

    std::fs::write(
        r_dir.join("test.R"),
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

// ---------------------------------------------------------------------------
// Multiple roxygen blocks
// ---------------------------------------------------------------------------

#[test]
fn test_roxygen_multiple_blocks() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();
    let r_dir = setup_r_package(directory);

    std::fs::write(
        r_dir.join("test.R"),
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
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/test.R:3:4
      |
    3 | #' any(is.na(x))
      |    ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_is_na
     --> R/test.R:8:4
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
    let r_dir = setup_r_package(directory);

    std::fs::write(
        r_dir.join("test.R"),
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

// ---------------------------------------------------------------------------
// Roxygen linting skipped for files outside an R package
// ---------------------------------------------------------------------------

#[test]
fn test_roxygen_skipped_outside_package() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // No DESCRIPTION, no R/ directory — just a plain R file
    std::fs::write(
        directory.join("test.R"),
        "\
#' Title
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

// ---------------------------------------------------------------------------
// \dontrun{}, \donttest{}, \dontshow{} wrappers are stripped
// ---------------------------------------------------------------------------

/// Code inside `\dontrun{}` is linted — the wrapper is stripped.
#[test]
fn test_roxygen_dontrun_linted() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();
    let r_dir = setup_r_package(directory);

    std::fs::write(
        r_dir.join("test.R"),
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
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/test.R:4:4
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

/// Code inside `\donttest{}` is linted — the wrapper is stripped.
#[test]
fn test_roxygen_donttest_linted() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();
    let r_dir = setup_r_package(directory);

    std::fs::write(
        r_dir.join("test.R"),
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
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/test.R:4:4
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

/// Code both inside and outside `\dontrun{}` is linted.
#[test]
fn test_roxygen_dontrun_with_surrounding_code() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();
    let r_dir = setup_r_package(directory);

    std::fs::write(
        r_dir.join("test.R"),
        "\
#' Title
#' @examples
#' any(is.na(x))
#' \\dontrun{
#' any(is.na(y))
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
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/test.R:3:4
      |
    3 | #' any(is.na(x))
      |    ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_is_na
     --> R/test.R:5:4
      |
    5 | #' any(is.na(y))
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
// @examples section stops at the next @tag
// ---------------------------------------------------------------------------

/// Code after `@return` (or any other tag) should NOT be linted as examples.
#[test]
fn test_roxygen_examples_stopped_by_tag() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();
    let r_dir = setup_r_package(directory);

    std::fs::write(
        r_dir.join("test.R"),
        "\
#' @title hi
#' @description
#' hello
#' @examples
#' any(is.na(x))
#' @return foo
#' any(is.na(x))
f <- function() 1
",
    )?;

    // Only the first any(is.na(x)) (inside @examples) should be reported.
    // The second one is under @return and is not R code.
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/test.R:5:4
      |
    5 | #' any(is.na(x))
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
// fix-roxygen = true applies fixes at the correct position
// ---------------------------------------------------------------------------

/// Multi-line roxygen example is correctly fixed in place.
#[test]
fn test_roxygen_fix_multiline() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();
    let r_dir = setup_r_package(directory);

    std::fs::write(
        r_dir.join("test.R"),
        "\
#' @title hi
#' @description
#' hello
#' @examples
#' 1 + 1
#' any(
#'   is.na(x)
#' )
#' 1 + 1
#' @return foo
f <- function() 1
",
    )?;

    std::fs::write(
        directory.join("jarl.toml"),
        "\
[lint]
fix-roxygen = true
",
    )?;

    Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--fix")
        .arg("--allow-no-vcs")
        .run();

    let fixed = std::fs::read_to_string(r_dir.join("test.R"))?;
    insta::assert_snapshot!(
        fixed,
        @"
    #' @title hi
    #' @description
    #' hello
    #' @examples
    #' 1 + 1
    #' anyNA(x)
    #' 1 + 1
    #' @return foo
    f <- function() 1
    "
    );

    Ok(())
}

/// Single-line roxygen example is correctly fixed in place.
#[test]
fn test_roxygen_fix_single_line() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();
    let r_dir = setup_r_package(directory);

    std::fs::write(
        r_dir.join("test.R"),
        "\
#' Title
#' @examples
#' 1 + 1
#' any(is.na(x))
#' 1 + 1
foo <- function(x) x
",
    )?;

    std::fs::write(
        directory.join("jarl.toml"),
        "\
[lint]
fix-roxygen = true
",
    )?;

    Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--fix")
        .arg("--allow-no-vcs")
        .run();

    let fixed = std::fs::read_to_string(r_dir.join("test.R"))?;
    insta::assert_snapshot!(
        fixed,
        @"
    #' Title
    #' @examples
    #' 1 + 1
    #' anyNA(x)
    #' 1 + 1
    foo <- function(x) x
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
    let r_dir = setup_r_package(directory);

    std::fs::write(
        r_dir.join("test.R"),
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
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/test.R:3:5
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
