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
fn test_min_r_version_from_description_in_package_layout() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: mypackage\nVersion: 1.0.0\nDepends: R (>= 4.6.0), utils, stats",
        ),
        ("R/test.R", "grep('a.*', x, value = TRUE)"),
    ])?;

    // grepv() only exists for R >= 4.5.0, and DESCRIPTION guarantees 4.6.0.
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
     --> R/test.R:1:1
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

    // Naming the file directly must resolve the same root.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("R/test.R")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: grepv
     --> R/test.R:1:1
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

/// Can't parse `Depends:` -> unknown version
#[test]
fn test_malformed_depends_r_does_not_panic() -> anyhow::Result<()> {
    for depends in ["R", "R >= 4.6.0", "R ()", "R (>= 4.6", "R )x("] {
        let case = CliTest::with_files([
            ("DESCRIPTION", ""),
            ("R/test.R", "grep('a.*', x, value = TRUE)"),
        ])?;
        case.write_file(
            "DESCRIPTION",
            &format!("Package: mypackage\nVersion: 1.0.0\nDepends: {depends}"),
        )?;

        let output = case.command().arg("check").arg(".").run();
        let rendered = format!("{output}");
        assert!(
            !rendered.contains("panicked"),
            "`Depends: {depends}` panicked:\n{rendered}"
        );
        // No usable version means version-gated rules stay off.
        assert!(
            !rendered.contains("grepv"),
            "`Depends: {depends}` should leave grepv disabled:\n{rendered}"
        );
    }

    Ok(())
}

#[test]
fn test_min_r_version_is_per_package_not_per_run() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "pkgA/DESCRIPTION",
            "Package: A\nVersion: 1.0.0\nDepends: R (>= 4.6.0)",
        ),
        (
            "pkgB/DESCRIPTION",
            "Package: B\nVersion: 1.0.0\nDepends: R (>= 3.6.0)",
        ),
        ("pkgA/R/a.R", "grep('a.*', x, value = TRUE)"),
        ("pkgB/R/b.R", "grep('a.*', x, value = TRUE)"),
    ])?;

    // Only pkgA has the R version high enough
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
     --> pkgA/R/a.R:1:1
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

    // `--min-r-version` overrides all Depends.
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
     --> pkgA/R/a.R:1:1
      |
    1 | grep('a.*', x, value = TRUE)
      | ---------------------------- `grep(..., value = TRUE)` can be simplified.
      |
      = help: Use `grepv(...)` instead.

    warning: grepv
     --> pkgB/R/b.R:1:1
      |
    1 | grep('a.*', x, value = TRUE)
      | ---------------------------- `grep(..., value = TRUE)` can be simplified.
      |
      = help: Use `grepv(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.
    2 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}
