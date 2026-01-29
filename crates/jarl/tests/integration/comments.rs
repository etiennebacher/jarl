use std::process::Command;
use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_jarl_ignore_node_suppression() -> anyhow::Result<()> {
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
fn test_jarl_ignore_without_space_after_hash() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
#jarl-ignore any_is_na: also valid without space
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
fn test_jarl_ignore_missing_explanation() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
# jarl-ignore any_is_na
any(is.na(x))
# jarl-ignore any_is_na:
any(is.na(x))
",
    )?;

    // Should lint because explanation is missing
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
fn test_jarl_ignore_invalid_rule() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
# jarl-ignore not_a_real_rule: some reason
any(is.na(x))
",
    )?;

    // Should lint because rule name is invalid
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
fn test_jarl_ignore_wrong_rule() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
# jarl-ignore any_duplicated: wrong rule for this violation
any(is.na(x))
",
    )?;

    // Should lint because we're suppressing a different rule
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
fn test_jarl_ignore_nested_in_call() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
foo(
  # jarl-ignore any_is_na: nested suppression
  any(is.na(x))
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
fn test_jarl_ignore_multiple_rules_multiple_comments() -> anyhow::Result<()> {
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
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_jarl_ignore_start_end() -> anyhow::Result<()> {
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

    // First and last should lint, middle two should be suppressed
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
fn test_jarl_ignore_start_end_in_function() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
f <- function() {
    # jarl-ignore-start any_is_na: internal section
    any(is.na(x))
    any(is.na(y))
    # jarl-ignore-end any_is_na

    any(is.na(z))
}
",
    )?;

    // Only the last one should lint
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
fn test_jarl_ignore_start_end_mismatched_rules() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
# jarl-ignore-start any_is_na: reason
any(is.na(x))
# jarl-ignore-end browser
any(is.na(y))
",
    )?;

    // Both should lint because the end doesn't match the start
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
fn test_jarl_ignore_start_end_unmatched_end() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
# jarl-ignore-end any_is_na
any(is.na(x))
",
    )?;

    // Should lint because end without start is ignored
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
fn test_jarl_ignore_file() -> anyhow::Result<()> {
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
fn test_jarl_ignore_file_multiple_rules() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "# jarl-ignore-file any_is_na: legacy patterns
# jarl-ignore-file assignment: uses = throughout
x = any(is.na(y))
z = any(is.na(w))
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
fn test_jarl_ignore_file_after_other_comments() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "# This is a header comment
# describing the file
# jarl-ignore-file any_is_na: reason here
any(is.na(x))
",
    )?;

    // Should work because jarl-ignore-file is before any R code
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
fn test_jarl_ignore_file_after_code_not_recognized() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
any(is.na(x))
# jarl-ignore-file any_is_na: too late
any(is.na(y))
",
    )?;

    // Should lint both because jarl-ignore-file is after R code
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
fn test_jarl_ignore_file_only_suppresses_specified_rule() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "# jarl-ignore-file any_is_na: only this rule
x = any(is.na(y))
",
    )?;

    // Should still lint assignment rule
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
fn test_nolint_not_recognized() -> anyhow::Result<()> {
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

    // Should lint all because old nolint format is not recognized
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
fn test_generated_by_not_recognized() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
# Generated by roxygen2: do not edit by hand
any(is.na(x))
",
    )?;

    // Should lint because Generated by is not recognized anymore
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
fn test_cascading_suppression() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
# jarl-ignore any_is_na: cascades to children
x <- any(is.na(y))
",
    )?;

    // The comment is attached to the binary expression (<-), but should
    // cascade to suppress the any(is.na()) call inside
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

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

    // The first one should be ignored, the second one should be reported.
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
