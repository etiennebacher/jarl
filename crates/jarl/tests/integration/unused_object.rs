use crate::helpers::{CliTest, CommandExt};

#[test]
fn test_exported_alias_not_flagged() -> anyhow::Result<()> {
    // `summarize_each <- summarise_each` is a typical alias-style export. The
    // RHS isn't a function literal, so the existing function-def filter
    // doesn't suppress it; we rely on the NAMESPACE export list instead.
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        ("NAMESPACE", "export(summarize_each)\n"),
        (
            "R/aliases.R",
            "summarise_each <- function(x) x\nsummarize_each <- summarise_each\n",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
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
fn test_unexported_alias_is_flagged() -> anyhow::Result<()> {
    // Same code, but no NAMESPACE export — `summarize_each` is dead.
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        ("NAMESPACE", ""),
        (
            "R/aliases.R",
            "summarise_each <- function(x) x\nsummarize_each <- summarise_each\n",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @r"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_object
     --> R/aliases.R:2:1
      |
    2 | summarize_each <- summarise_each
      | -------------- Object `summarize_each` is defined but never used.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}
