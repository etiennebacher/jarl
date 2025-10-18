use git2::*;
use jarl_core::fs::relativize_path;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;
use crate::helpers::create_commit;

#[test]
fn test_clean_git_repo() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let file_path = PathBuf::from(relativize_path(directory.join(test_path)));
    let test_contents = "any(is.na(x))";
    std::fs::write(&file_path, test_contents)?;

    let repo = Repository::init(directory)?;
    create_commit(file_path, repo)?;

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
fn test_dirty_git_repo_does_not_block_lint() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    let _ = Repository::init(directory)?;

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
fn test_dirty_git_repo_blocks_fix() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    let _ = Repository::init(directory)?;

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
fn test_dirty_git_repo_allow_dirty() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    let _ = Repository::init(directory)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--fix")
            .arg("--allow-dirty")
            .run()
            .normalize_os_executable_name()
    );
    Ok(())
}
