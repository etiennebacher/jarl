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
                        .normalize_temp_paths(),
                    @r"
success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: Invalid configuration in [TEMP_DIR]/jarl.toml:
Cannot specify both `skipped-functions` and `extend-skipped-functions` in `[lint.duplicated-arguments]`.
"
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
                        .normalize_temp_paths(),
                    @r"
success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: Invalid configuration in [TEMP_DIR]/jarl.toml:
Cannot specify both `stopping-functions` and `extend-stopping-functions` in `[lint.unreachable-code]`.
"
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
            .normalize_temp_paths(),
        @r#"
success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: Failed to parse [TEMP_DIR]/jarl.toml:
TOML parse error at line 5, column 1
  |
5 | unknown-option = ["list"]
  | ^^^^^^^^^^^^^^
unknown field `unknown-option`, expected `skipped-functions` or `extend-skipped-functions`

"#
    );

    Ok(())
}

#[test]
fn test_assignment_unknown_field_is_error() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint.assignment]
unknown-option = "foo"
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
            .normalize_temp_paths(),
        @r#"
success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: Failed to parse [TEMP_DIR]/jarl.toml:
TOML parse error at line 3, column 1
  |
3 | unknown-option = "foo"
  | ^^^^^^^^^^^^^^
unknown field `unknown-option`, expected `operator`

"#
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
            .normalize_temp_paths(),
        @r#"
success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: Failed to parse [TEMP_DIR]/jarl.toml:
TOML parse error at line 5, column 1
  |
5 | unknown-option = ["stop"]
  | ^^^^^^^^^^^^^^
unknown field `unknown-option`, expected `stopping-functions` or `extend-stopping-functions`

"#
    );

    Ok(())
}
