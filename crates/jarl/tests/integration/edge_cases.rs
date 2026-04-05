use crate::helpers::{CliTest, CommandExt};

// This collects edge cases and runs them with all rules to ensure that we didn't
// fix just one particular rule but left errors in another one;

// https://github.com/etiennebacher/jarl/issues/416
#[test]
fn test_jarl_break_and_next_kw_as_call() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.R",
        "
for (i in 1:3) {
    break()
}
for (i in 1:3) {
    next()
}",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
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

// Ensure dashes under violating code are correctly aligned with tabs
#[test]
fn test_jarl_with_tabs() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "\t\tany(is.na(x))")?;

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
     --> test.R:1:9
      |
    1 |         any(is.na(x))
      |         ------------- `any(is.na(...))` is inefficient.
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
