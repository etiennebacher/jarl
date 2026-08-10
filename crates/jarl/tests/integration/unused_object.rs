use crate::helpers::{CliTest, CommandExt};

#[test]
fn test_relative_source_resolves_next_to_the_file() -> anyhow::Result<()> {
    // `main.R` redefines `x`, sources a helper that also defines `x`, then
    // reads `x`. R's `source()` evaluates the helper in the calling
    // environment, so the helper's `x` wins and `main.R`'s own `x <- 2` is
    // dead — but only if the sourced path resolves, which makes the
    // diagnostic a direct probe of that resolution.
    //
    // The linted path (`sub/main.R`) is relative to the CWD, so resolution
    // walks anchors from the file's own directory upwards. Here the helper
    // sits next to the file, i.e. the first anchor.
    let case = CliTest::with_files([
        ("sub/helper.R", "x <- 1\n"),
        ("sub/main.R", "x <- 2\nsource(\"helper.R\")\nprint(x)\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("sub/main.R")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_object
     --> sub/main.R:1:1
      |
    1 | x <- 2
      | - Object `x` is defined but never used.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_relative_source_resolves_at_an_ancestor_directory() -> anyhow::Result<()> {
    // Same, but the helper only exists one anchor up (at the CWD), the way a
    // script run from the project root would find it.
    let case = CliTest::with_files([
        ("helper.R", "x <- 1\n"),
        ("sub/main.R", "x <- 2\nsource(\"helper.R\")\nprint(x)\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("sub/main.R")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_object
     --> sub/main.R:1:1
      |
    1 | x <- 2
      | - Object `x` is defined but never used.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_relative_source_that_resolves_nowhere_is_ignored() -> anyhow::Result<()> {
    // No anchor between the file and the CWD has `nope.R`: resolution fails
    // and `print(x)` reads `main.R`'s own binding, so nothing is reported.
    let case = CliTest::with_files([
        ("helper.R", "x <- 1\n"),
        ("sub/main.R", "x <- 2\nsource(\"nope.R\")\nprint(x)\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("sub/main.R")
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
