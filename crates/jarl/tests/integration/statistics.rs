use std::process::Command;

use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_stats() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
";
    std::fs::write(directory.join(test_path), test_contents)?;

    let test_path_2 = "test2.R";
    let test_contents_2 = "mean(x <- 1)";
    std::fs::write(directory.join(test_path_2), test_contents_2)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--statistics")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_stats_no_violation() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "x <- 1";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--statistics")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_hint_stats_arg() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--output-format")
            .arg("concise")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_hint_stats_arg_with_envvar() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--output-format")
            .arg("concise")
            .env("JARL_N_VIOLATIONS_HINT_STAT", "25")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}
