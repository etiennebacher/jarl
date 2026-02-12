use std::process::Command;

use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_duplicated_arguments_both_skipped_and_extend_is_error() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]

[lint.duplicated-arguments]
skipped-functions = ["list"]
extend-skipped-functions = ["my_fun"]
"#,
    )?;

    std::fs::write(directory.join("test.R"), "list(a = 1, a = 2)")?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
    );

    Ok(())
}

#[test]
fn test_unreachable_code_both_stopping_and_extend_is_error() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]

[lint.unreachable-code]
stopping-functions = ["stop"]
extend-stopping-functions = ["my_stop"]
"#,
    )?;

    std::fs::write(
        directory.join("test.R"),
        r#"
foo <- function() {
  stop("error")
  1 + 1
}
"#,
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
    );

    Ok(())
}

#[test]
fn test_duplicated_arguments_unknown_field_is_error() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]

[lint.duplicated-arguments]
unknown-option = ["list"]
"#,
    )?;

    std::fs::write(directory.join("test.R"), "list(a = 1, a = 2)")?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
    );

    Ok(())
}

#[test]
fn test_unreachable_code_unknown_field_is_error() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]

[lint.unreachable-code]
unknown-option = ["stop"]
"#,
    )?;

    std::fs::write(directory.join("test.R"), "x <- 1")?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
    );

    Ok(())
}
