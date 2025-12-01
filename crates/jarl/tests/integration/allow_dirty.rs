use git2::*;
use std::process::Command;
use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;
use crate::helpers::create_commit;

#[test]
fn test_clean_git_repo() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // In other tests for `--allow-*`, I use "demos/test.R" just to check that
    // VCS detection works fine in subfolders.
    // Here, `create_commit()` must take a relative path which is annoying to
    // extract with "demos/test.R".
    let test_path = "test.R";
    let file_path = directory.join(test_path);
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

    let test_path = "demos/test.R";
    let test_contents = "any(is.na(x))";
    std::fs::create_dir_all(directory.join("demos"))?;
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

    // Ensure that the message is printed only once and not once per file
    // https://github.com/etiennebacher/jarl/issues/135
    let test_path = "demos/test.R";
    let test_path_2 = "demos/test_2.R";
    let test_contents = "any(is.na(x))";
    std::fs::create_dir_all(directory.join("demos"))?;
    std::fs::write(directory.join(test_path), test_contents)?;
    std::fs::write(directory.join(test_path_2), test_contents)?;

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

    let test_path = "demos/test.R";
    let test_contents = "any(is.na(x))";
    std::fs::create_dir_all(directory.join("demos"))?;
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

#[test]
fn test_mixed_dirty_status_blocks_fix() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // Create two subdirectories
    let clean_subdir = directory.join("clean");
    let dirty_subdir = directory.join("dirty");
    std::fs::create_dir_all(&clean_subdir)?;
    std::fs::create_dir_all(&dirty_subdir)?;

    // Create test files in both subdirs
    let test_contents = "any(is.na(x))";
    std::fs::write(clean_subdir.join("test.R"), test_contents)?;
    std::fs::write(dirty_subdir.join("test.R"), test_contents)?;

    // Each subdir is a separate repo
    let clean_repo = Repository::init(clean_subdir.clone())?;
    let _ = Repository::init(dirty_subdir)?;

    // Make only one of these two repos clean, leaving the other dirty
    create_commit(clean_subdir.join("test.R"), clean_repo)?;

    // Try to fix both subdirs - should fail because one has dirty changes
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
fn test_two_clean_subdirs() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // Create two subdirectories
    let subdir_1 = directory.join("clean");
    let subdir_2 = directory.join("dirty");
    std::fs::create_dir_all(&subdir_1)?;
    std::fs::create_dir_all(&subdir_2)?;

    // Create test files in both subdirs
    let test_contents = "any(is.na(x))";
    std::fs::write(subdir_1.join("test.R"), test_contents)?;
    std::fs::write(subdir_2.join("test.R"), test_contents)?;

    // Each subdir is a separate repo
    let repo_1 = Repository::init(subdir_1.clone())?;
    let repo_2 = Repository::init(subdir_2.clone())?;

    // Both repos are clean
    create_commit(subdir_1.join("test.R"), repo_1)?;
    create_commit(subdir_2.join("test.R"), repo_2)?;

    // Parent folder is not a git repo, but all files in subfolders are covered
    // by Git (even if the repos are different).
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
