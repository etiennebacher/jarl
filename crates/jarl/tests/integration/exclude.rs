use std::process::Command;

use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

/// Excluded files should still contribute symbol usages for cross-file
/// analysis (e.g. unused_function). If `foo.R` calls `f()` and `foo2.R`
/// defines `f()`, excluding `foo.R` should NOT cause `f` to be reported
/// as unused.
#[test]
fn test_excluded_file_contributes_symbols() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // Set up a minimal R package
    std::fs::write(directory.join("DESCRIPTION"), "")?;
    std::fs::write(directory.join("NAMESPACE"), "")?;
    std::fs::create_dir(directory.join("R"))?;

    // foo2.R defines f()
    std::fs::write(directory.join("R/foo2.R"), "f <- function() 1 + 1\n")?;
    // foo.R calls f() — this file will be excluded
    std::fs::write(directory.join("R/foo.R"), "f()\n")?;

    // Exclude foo.R via jarl.toml
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
select = ["unused_function"]
exclude = ["R/foo.R"]
"#,
    )?;

    // f() should NOT be reported as unused because excluded foo.R calls it
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
fn test_excluded_file_not_in_r_folder_contributes_symbols() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // Set up a minimal R package
    std::fs::write(directory.join("DESCRIPTION"), "")?;
    std::fs::write(directory.join("NAMESPACE"), "")?;
    std::fs::create_dir(directory.join("R"))?;
    std::fs::create_dir(directory.join("tests"))?;

    // foo2.R defines f()
    std::fs::write(directory.join("R/foo.R"), "f <- function() 1 + 1\n")?;
    // foo.R calls f() — this file will be excluded
    std::fs::write(directory.join("tests/foo.R"), "f()\n")?;

    // Exclude foo.R via jarl.toml
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
select = ["unused_function"]
exclude = ["R/foo.R"]
"#,
    )?;

    // f() should NOT be reported as unused because excluded foo.R calls it
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

/// Same for explicitly included files
#[test]
fn test_included_file_contributes_symbols() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // Set up a minimal R package
    std::fs::write(directory.join("DESCRIPTION"), "")?;
    std::fs::write(directory.join("NAMESPACE"), "")?;
    std::fs::create_dir(directory.join("R"))?;

    // foo2.R defines f()
    std::fs::write(directory.join("R/foo2.R"), "f <- function() 1 + 1\n")?;
    // foo.R calls f() — this file will be excluded
    std::fs::write(directory.join("R/foo.R"), "f()\n")?;

    // Exclude foo.R via jarl.toml
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
select = ["unused_function"]
include = ["R/foo2.R"]
"#,
    )?;

    // f() should NOT be reported as unused because excluded foo.R calls it
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

/// Excluded files should still contribute assignments for
/// duplicated_function_definition. If `foo.R` and `foo2.R` both define `f()`,
/// excluding `foo.R` should still detect the duplicate.
#[test]
fn test_excluded_file_contributes_assignments() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(directory.join("DESCRIPTION"), "")?;
    std::fs::write(directory.join("NAMESPACE"), "")?;
    std::fs::create_dir(directory.join("R"))?;

    // Both files define f()
    std::fs::write(directory.join("R/foo.R"), "f <- function() 1\n")?;
    std::fs::write(directory.join("R/foo2.R"), "f <- function() 2\n")?;

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
select = ["duplicated_function_definition"]
exclude = ["R/foo.R"]
"#,
    )?;

    // foo2.R should report f as duplicated (other definition in excluded foo.R)
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @r"
    success: false
    exit_code: 1
    ----- stdout -----
    warning: duplicated_function_definition
     --> R/foo2.R:1:1
      |
    1 | f <- function() 2
      | - `f` is defined more than once in this package.
      |
      = help: Other definition at R/foo.R:1:1


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_included_file_contributes_assignments() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(directory.join("DESCRIPTION"), "")?;
    std::fs::write(directory.join("NAMESPACE"), "")?;
    std::fs::create_dir(directory.join("R"))?;

    // Both files define f()
    std::fs::write(directory.join("R/foo.R"), "f <- function() 1\n")?;
    std::fs::write(directory.join("R/foo2.R"), "f <- function() 2\n")?;

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
select = ["duplicated_function_definition"]
include = ["R/foo2.R"]
"#,
    )?;

    // foo2.R should report f as duplicated (other definition in excluded foo.R)
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @r"
    success: false
    exit_code: 1
    ----- stdout -----
    warning: duplicated_function_definition
     --> R/foo2.R:1:1
      |
    1 | f <- function() 2
      | - `f` is defined more than once in this package.
      |
      = help: Other definition at R/foo.R:1:1


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}
