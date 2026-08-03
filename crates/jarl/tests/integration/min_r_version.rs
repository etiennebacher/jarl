use crate::helpers::{CliTest, CommandExt};

#[test]
fn test_min_r_version_from_cli_only() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "grep('a.*', x, value = TRUE)")?;

    // grepv() rule only exists for R >= 4.5.

    // By default, if we don't know the min R version, we disable rules that
    // only exist starting from a specific version.
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

    // This should not report a lint (the project could be using 4.4.0 so
    // grepv() wouldn't exist).
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--min-r-version")
            .arg("4.4.0")
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
    // This should report a lint.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--min-r-version")
            .arg("4.6.0")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: grepv
     --> test.R:1:1
      |
    1 | grep('a.*', x, value = TRUE)
      | ---------------------------- `grep(..., value = TRUE)` can be simplified.
      |
      = help: Use `grepv(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_min_r_version_from_description_only() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "grep('a.*', x, value = TRUE)")?;

    // grepv() rule only exists for R >= 4.5.0

    // This should not report a lint (the project could be using 4.4.0 so
    // grepv() wouldn't exist).
    case.write_file(
        "DESCRIPTION",
        r#"Package: mypackage
Version: 1.0.0
Depends: R (>= 4.4.0), utils, stats"#,
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

    // This should report a lint.
    case.write_file(
        "DESCRIPTION",
        r#"Package: mypackage
Version: 1.0.0
Depends: R (>= 4.6.0), utils, stats"#,
    )?;
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
    warning: grepv
     --> test.R:1:1
      |
    1 | grep('a.*', x, value = TRUE)
      | ---------------------------- `grep(..., value = TRUE)` can be simplified.
      |
      = help: Use `grepv(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_min_r_version_from_description_constraints() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "grep('a.*', x, value = TRUE)")?;

    // A strict lower bound still guarantees that grepv() is available.
    case.write_file(
        "DESCRIPTION",
        r#"Package: mypackage
Version: 1.0.0
Depends: R (> 4.5.0)"#,
    )?;
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
    warning: grepv
     --> test.R:1:1
      |
    1 | grep('a.*', x, value = TRUE)
      | ---------------------------- `grep(..., value = TRUE)` can be simplified.
      |
      = help: Use `grepv(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    // Multiple lower bounds use the strictest version, regardless of order.
    case.write_file(
        "DESCRIPTION",
        r#"Package: mypackage
Version: 1.0.0
Depends: R (>= 4.4.0), R (>= 4.6.0)"#,
    )?;
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
    warning: grepv
     --> test.R:1:1
      |
    1 | grep('a.*', x, value = TRUE)
      | ---------------------------- `grep(..., value = TRUE)` can be simplified.
      |
      = help: Use `grepv(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    // An upper bound cannot establish a minimum version, so version-gated
    // rules remain disabled without aborting the check.
    case.write_file(
        "DESCRIPTION",
        r#"Package: mypackage
Version: 1.0.0
Depends: R (< 5.0.0)"#,
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

    // R-devel revision constraints cannot be represented by Jarl's numeric
    // version tuple and are ignored rather than causing the command to fail.
    case.write_file(
        "DESCRIPTION",
        r#"Package: mypackage
Version: 1.0.0
Depends: R (>= r56550)"#,
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
