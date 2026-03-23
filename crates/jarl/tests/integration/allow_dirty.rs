use crate::helpers::CliTest;
use crate::helpers::CommandExt;
use crate::helpers::create_commit;
use crate::helpers::git_init;

#[test]
fn test_clean_git_repo() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "any(is.na(x))")?;

    git_init(case.root())?;
    create_commit(&case.root().join("test.R"), case.root())?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--fix")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );
    Ok(())
}

#[test]
fn test_dirty_git_repo_does_not_block_lint() -> anyhow::Result<()> {
    let case = CliTest::with_file("demos/test.R", "any(is.na(x))")?;

    git_init(case.root())?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> demos/test.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );
    Ok(())
}

#[test]
fn test_dirty_git_repo_blocks_fix() -> anyhow::Result<()> {
    // Ensure that the message is printed only once and not once per file
    // https://github.com/etiennebacher/jarl/issues/135
    let case = CliTest::with_files([
        ("demos/test.R", "any(is.na(x))"),
        ("demos/test_2.R", "any(is.na(x))"),
    ])?;

    git_init(case.root())?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--fix")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    Error: `jarl check --fix` can potentially perform destructive changes but the working directory of this project has uncommitted changes, so no fixes were applied.
    To apply the fixes, either add `--allow-dirty` to the call, or commit the changes to these files:

      * demos/ (dirty)
    "
    );
    Ok(())
}

#[test]
fn test_dirty_git_repo_allow_dirty() -> anyhow::Result<()> {
    let case = CliTest::with_file("demos/test.R", "any(is.na(x))")?;

    git_init(case.root())?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--fix")
            .arg("--allow-dirty")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );
    Ok(())
}

#[test]
fn test_mixed_dirty_status_blocks_fix() -> anyhow::Result<()> {
    let case = CliTest::new()?;

    // Create two subdirectories with separate git repos
    let clean_subdir = case.root().join("clean");
    let dirty_subdir = case.root().join("dirty");
    std::fs::create_dir_all(&clean_subdir)?;
    std::fs::create_dir_all(&dirty_subdir)?;

    // Create test files in both subdirs
    let test_contents = "any(is.na(x))";
    std::fs::write(clean_subdir.join("test.R"), test_contents)?;
    std::fs::write(dirty_subdir.join("test.R"), test_contents)?;

    // Each subdir is a separate repo
    git_init(&clean_subdir)?;
    git_init(&dirty_subdir)?;

    // Make only one of these two repos clean, leaving the other dirty
    create_commit(&clean_subdir.join("test.R"), &clean_subdir)?;

    // Try to fix both subdirs - should fail because one has dirty changes
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--fix")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    Error: `jarl check --fix` can potentially perform destructive changes but the working directory of this project has uncommitted changes, so no fixes were applied.
    To apply the fixes, either add `--allow-dirty` to the call, or commit the changes to these files:

      * test.R (dirty)
    "
    );
    Ok(())
}

#[test]
fn test_two_clean_subdirs() -> anyhow::Result<()> {
    let case = CliTest::new()?;

    // Create two subdirectories with separate git repos
    let subdir_1 = case.root().join("clean");
    let subdir_2 = case.root().join("dirty");
    std::fs::create_dir_all(&subdir_1)?;
    std::fs::create_dir_all(&subdir_2)?;

    // Create test files in both subdirs
    let test_contents = "any(is.na(x))";
    std::fs::write(subdir_1.join("test.R"), test_contents)?;
    std::fs::write(subdir_2.join("test.R"), test_contents)?;

    // Each subdir is a separate repo
    git_init(&subdir_1)?;
    git_init(&subdir_2)?;

    // Both repos are clean
    create_commit(&subdir_1.join("test.R"), &subdir_1)?;
    create_commit(&subdir_2.join("test.R"), &subdir_2)?;

    // Parent folder is not a git repo, but all files in subfolders are covered
    // by Git (even if the repos are different).
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--fix")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );
    Ok(())
}
