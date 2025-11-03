use std::process::Command;
use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_no_git_repo_does_not_block_lint() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "demos/test.R";
    let test_contents = "any(is.na(x))";
    std::fs::create_dir_all(directory.join("demos"))?;
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--fix")
            .run()
            .normalize_os_executable_name()
    );
    Ok(())
}

#[test]
fn test_no_git_repo_blocks_fix() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "demos/test.R";
    let test_contents = "any(is.na(x))";
    std::fs::create_dir_all(directory.join("demos"))?;
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--fix")
            .run()
            .normalize_os_executable_name()
    );
    Ok(())
}

#[test]
fn test_no_git_repo_allow_no_vcs() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "demos/test.R";
    let test_contents = "any(is.na(x))";
    std::fs::create_dir_all(directory.join("demos"))?;
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--fix")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );
    Ok(())
}
