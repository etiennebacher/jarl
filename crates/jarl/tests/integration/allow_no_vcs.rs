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

    // Ensure that the message is printed only once and not once per file
    // https://github.com/etiennebacher/jarl/issues/135
    let test_path = "demos/test.R";
    let test_path_2 = "demos/test_2.R";
    let test_contents = "any(is.na(x))";
    std::fs::create_dir_all(directory.join("demos"))?;
    std::fs::write(directory.join(test_path), test_contents)?;
    std::fs::write(directory.join(test_path_2), test_contents)?;

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

#[test]
fn test_mixed_vcs_coverage_blocks_fix() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // Create two subdirectories
    let git_subdir = directory.join("git_covered");
    let no_git_subdir = directory.join("not_covered");
    std::fs::create_dir_all(&git_subdir)?;
    std::fs::create_dir_all(&no_git_subdir)?;

    // Create test files in both subdirs
    let test_contents = "any(is.na(x))";
    std::fs::write(git_subdir.join("test.R"), test_contents)?;
    std::fs::write(no_git_subdir.join("test.R"), test_contents)?;

    // Only initialize git in one subdir
    let _ = git2::Repository::init(&git_subdir)?;

    // Try to fix both subdirs - should fail because one is not in VCS
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
