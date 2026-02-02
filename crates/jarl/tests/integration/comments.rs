use std::process::Command;
use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_jarl_ignore_inline_suppression() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
# jarl-ignore any_is_na: legacy code
any(is.na(x))
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_jarl_ignore_file_suppression() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "# jarl-ignore-file any_is_na: this file has many false positives
any(is.na(x))
any(is.na(y))
any(is.na(z))
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_jarl_ignore_region_suppression() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
any(is.na(x))

# jarl-ignore-start any_is_na: debugging section
any(is.na(y))
any(is.na(z))
# jarl-ignore-end any_is_na

any(is.na(w))
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_jarl_ignore_cascading_suppression() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
# jarl-ignore any_is_na: cascades to children
x <- function(x) {
    any(is.na(y))
}
any(is.na(y))
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_jarl_ignore_multiple_rules_with_extend_select() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
# jarl-ignore any_is_na: first rule
# jarl-ignore assignment: second rule
x = any(is.na(y))
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--extend-select")
            .arg("assignment")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_jarl_ignore_nested_in_call_second_argument() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
foo(
  first_arg,
  # jarl-ignore implicit_assignment: suppressing second arg
  x <- 1
)
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_nolint_format_not_recognized() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
# nolint
any(is.na(x))
# nolint: any_is_na
any(is.na(y))
# nolint start
any(is.na(z))
# nolint end
",
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}
