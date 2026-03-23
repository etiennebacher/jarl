use crate::helpers::{CliTest, CommandExt, git_init};

#[test]
fn test_no_git_repo_does_not_block_lint() -> anyhow::Result<()> {
    let case = CliTest::with_file("demos/test.R", "any(is.na(x))")?;

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
    Error: `jarl check --fix` can potentially perform destructive changes but no Version Control System (e.g. Git) was found on this project, so no fixes were applied.
    Add `--allow-no-vcs` to the call to apply the fixes.
    "
    );
    Ok(())
}

#[test]
fn test_no_git_repo_blocks_fix() -> anyhow::Result<()> {
    // Ensure that the message is printed only once and not once per file
    // https://github.com/etiennebacher/jarl/issues/135
    let case = CliTest::with_files([
        ("demos/test.R", "any(is.na(x))"),
        ("demos/test_2.R", "any(is.na(x))"),
    ])?;

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
    Error: `jarl check --fix` can potentially perform destructive changes but no Version Control System (e.g. Git) was found on this project, so no fixes were applied.
    Add `--allow-no-vcs` to the call to apply the fixes.
    "
    );
    Ok(())
}

#[test]
fn test_no_git_repo_allow_no_vcs() -> anyhow::Result<()> {
    let case = CliTest::with_file("demos/test.R", "any(is.na(x))")?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--fix")
            .arg("--allow-no-vcs")
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
fn test_mixed_vcs_coverage_blocks_fix() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("git_covered/test.R", "any(is.na(x))"),
        ("not_covered/test.R", "any(is.na(x))"),
    ])?;

    // Only initialize git in one subdir
    git_init(&case.root().join("git_covered"))?;

    // Try to fix both subdirs - should fail because one is not in VCS
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
    Error: `jarl check --fix` can potentially perform destructive changes but no Version Control System (e.g. Git) was found on this project, so no fixes were applied.
    Add `--allow-no-vcs` to the call to apply the fixes.
    "
    );
    Ok(())
}
