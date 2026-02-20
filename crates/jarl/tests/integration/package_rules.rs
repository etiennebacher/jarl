use std::process::Command;

use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

/// Create a minimal R package structure under `dir`.
///
/// Returns the path to the `R/` subdirectory.
fn create_package(dir: &std::path::Path) -> std::path::PathBuf {
    std::fs::write(
        dir.join("DESCRIPTION"),
        "Package: testpkg\nVersion: 0.1.0\n",
    )
    .unwrap();
    let r_dir = dir.join("R");
    std::fs::create_dir(&r_dir).unwrap();
    r_dir
}

// ── Same-file duplicate ───────────────────────────────────────────────────

#[test]
fn test_same_file_duplicate_assignment() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();
    let r_dir = create_package(directory);

    std::fs::write(
        r_dir.join("foo.R"),
        "foo <- function() 1\nfoo <- function() 2\n",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("duplicated_function_definition")
            .run()
            .normalize_os_executable_name(),
        @"
    success: false
    exit_code: 1
    ----- stdout -----
    warning: duplicated_function_definition
     --> R/foo.R:2:1
      |
    2 | foo <- function() 2
      | --- `foo` is defined more than once in this package.
      |
      = help: other definition at R/foo.R:1:1

    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

// ── Cross-file duplicate ──────────────────────────────────────────────────

#[test]
fn test_cross_file_duplicate_assignment() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();
    let r_dir = create_package(directory);

    // aaa.R is alphabetically first → first definition (not flagged)
    std::fs::write(r_dir.join("aaa.R"), "foo <- function() 1\n")?;
    // bbb.R is alphabetically second → duplicate (flagged)
    std::fs::write(r_dir.join("bbb.R"), "foo <- function() 2\n")?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("duplicated_function_definition")
            .run()
            .normalize_os_executable_name(),
        @"
    success: false
    exit_code: 1
    ----- stdout -----
    warning: duplicated_function_definition
     --> R/bbb.R:1:1
      |
    1 | foo <- function() 2
      | --- `foo` is defined more than once in this package.
      |
      = help: other definition at R/aaa.R:1:1

    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

// ── Not inside a package (no DESCRIPTION) ────────────────────────────────

#[test]
fn test_no_lint_outside_package() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // No DESCRIPTION file → not a package
    let r_dir = directory.join("R");
    std::fs::create_dir(&r_dir)?;
    std::fs::write(r_dir.join("foo.R"), "foo <- 1\nfoo <- 2\n")?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("duplicated_function_definition")
            .run()
            .normalize_os_executable_name(),
        @"
    success: true
    exit_code: 0
    ----- stdout -----
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

// ── Rule can be ignored ───────────────────────────────────────────────────

#[test]
fn test_ignore_duplicated_function_definition() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();
    let r_dir = create_package(directory);

    std::fs::write(r_dir.join("foo.R"), "foo <- 1\nfoo <- 2\n")?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--ignore")
            .arg("duplicated_function_definition")
            .run()
            .normalize_os_executable_name(),
        @"
    success: true
    exit_code: 0
    ----- stdout -----
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}
