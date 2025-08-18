use std::process::Command;

use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_no_r_files() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_parsing_error() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let path = "test.R";
    std::fs::write(directory.join(path), "f <-")?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_no_lints() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let path = "test.R";
    std::fs::write(directory.join(path), "any(x)")?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_one_lint() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_several_lints_one_file() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))\nany(duplicated(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_several_lints_several_files() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    let test_path_2 = "test2.R";
    let test_contents_2 = "any(duplicated(x))";
    std::fs::write(directory.join(test_path_2), test_contents_2)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_not_all_fixable_lints() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    let test_path_2 = "test2.R";
    let test_contents_2 = "list(x = 1, x = 2)";
    std::fs::write(directory.join(test_path_2), test_contents_2)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_minimum_r_version() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    // grepv() rule only exists for R >= 4.5.0
    let test_contents = "grep('a', x, value = TRUE)";
    std::fs::write(directory.join(test_path), test_contents)?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("--min-r-version")
            .arg("4.4")
            .run()
            .normalize_os_executable_name()
    );
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("--min-r-version")
            .arg("4.6")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}
