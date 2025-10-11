use std::fs;
use std::process::Command;
use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_leading_trailing_comment() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "# a comment\nany(is.na(x))\n# another comment";
    std::fs::write(directory.join(test_path), test_contents)?;
    let _ = &mut Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--fix")
        .run()
        .normalize_os_executable_name();
    insta::assert_snapshot!(fs::read_to_string(directory.join(test_path))?);

    Ok(())
}

#[test]
fn test_comment_on_same_line() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x)) # a comment";
    std::fs::write(directory.join(test_path), test_contents)?;
    let _ = &mut Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--fix")
        .run()
        .normalize_os_executable_name();
    insta::assert_snapshot!(fs::read_to_string(directory.join(test_path))?);

    Ok(())
}

#[test]
fn test_multiline_diagnostic_with_comments() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
any(
  # first comment
  is.na(
    # second comment
    x
  )
)";
    std::fs::write(directory.join(test_path), test_contents)?;
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
fn test_multiline_fix_with_comments() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
any(
  # first comment
  is.na(
    # second comment
    x
  )
)";
    std::fs::write(directory.join(test_path), test_contents)?;

    // Ideally, we should return a diagnostic here. This is not ideal, but
    // better this than mishandling this type of code.
    // TODO: when https://github.com/etiennebacher/flir2/issues/97 is fixed,
    // check that --fix doesn't destroy comments.
    let _ = &mut Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--fix")
        .run()
        .normalize_os_executable_name();
    insta::assert_snapshot!(fs::read_to_string(directory.join(test_path))?);

    Ok(())
}
