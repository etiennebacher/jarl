use std::process::Command;
use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_quotes_is_opt_in() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(directory.join("jarl.toml"), "[lint]\n")?;
    std::fs::write(directory.join("test.R"), "'x'")?;

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

#[test]
fn test_quotes_default_from_cli_selection() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(directory.join("jarl.toml"), "[lint]\n")?;
    std::fs::write(directory.join("test.R"), "'x'")?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("quotes")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: quotes
     --> test.R:1:1
      |
    1 | 'x'
      | --- Only use double-quotes.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_quotes_single_quote_from_toml() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(directory.join("test.R"), "\"x\"")?;
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
extend-select = ["quotes"]

[lint.quotes]
quote = "single"
"#,
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @r#"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: quotes
     --> test.R:1:1
      |
    1 | "x"
      | --- Only use single-quotes.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "#
    );

    Ok(())
}
