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
