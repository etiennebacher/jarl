use crate::helpers::{CliTest, CommandExt};

/// Excluded files should still contribute symbol usages for cross-file
/// analysis (e.g. unused_function). If `foo.R` calls `f()` and `foo2.R`
/// defines `f()`, excluding `foo.R` should NOT cause `f` to be reported
/// as unused.
#[test]
fn test_excluded_file_contributes_symbols() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("DESCRIPTION", ""),
        ("NAMESPACE", ""),
        ("R/foo2.R", "f <- function() 1 + 1\n"),
        ("R/foo.R", "f()\n"),
        (
            "jarl.toml",
            r#"
[lint]
select = ["unused_function"]
exclude = ["R/foo.R"]
"#,
        ),
    ])?;

    // f() should NOT be reported as unused because excluded foo.R calls it
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

#[test]
fn test_excluded_file_not_in_r_folder_contributes_symbols() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("DESCRIPTION", ""),
        ("NAMESPACE", ""),
        ("R/foo.R", "f <- function() 1 + 1\n"),
        ("tests/foo.R", "f()\n"),
        (
            "jarl.toml",
            r#"
[lint]
select = ["unused_function"]
exclude = ["R/foo.R"]
"#,
        ),
    ])?;

    // f() should NOT be reported as unused because excluded foo.R calls it
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

/// Same for explicitly included files
#[test]
fn test_included_file_contributes_symbols() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("DESCRIPTION", ""),
        ("NAMESPACE", ""),
        ("R/foo2.R", "f <- function() 1 + 1\n"),
        ("R/foo.R", "f()\n"),
        (
            "jarl.toml",
            r#"
[lint]
select = ["unused_function"]
include = ["R/foo2.R"]
"#,
        ),
    ])?;

    // f() should NOT be reported as unused because excluded foo.R calls it
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

/// Excluded files should still contribute assignments for
/// duplicated_function_definition. If `foo.R` and `foo2.R` both define `f()`,
/// excluding `foo.R` should still detect the duplicate.
#[test]
fn test_excluded_file_contributes_assignments() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("DESCRIPTION", ""),
        ("NAMESPACE", ""),
        ("R/foo.R", "f <- function() 1\n"),
        ("R/foo2.R", "f <- function() 2\n"),
        (
            "jarl.toml",
            r#"
[lint]
select = ["duplicated_function_definition"]
exclude = ["R/foo.R"]
"#,
        ),
    ])?;

    // foo2.R should report f as duplicated (other definition in excluded foo.R)
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: duplicated_function_definition
     --> R/foo2.R:1:1
      |
    1 | f <- function() 2
      | - `f` is defined more than once in this package.
      |
      = help: Other definition at R/foo.R:1:1


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_included_file_contributes_assignments() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("DESCRIPTION", ""),
        ("NAMESPACE", ""),
        ("R/foo.R", "f <- function() 1\n"),
        ("R/foo2.R", "f <- function() 2\n"),
        (
            "jarl.toml",
            r#"
[lint]
select = ["duplicated_function_definition"]
include = ["R/foo2.R"]
"#,
        ),
    ])?;

    // foo2.R should report f as duplicated (other definition in excluded foo.R)
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: duplicated_function_definition
     --> R/foo2.R:1:1
      |
    1 | f <- function() 2
      | - `f` is defined more than once in this package.
      |
      = help: Other definition at R/foo.R:1:1


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

// Files whose first non-blank line is a `# Generated by ...` comment
// should be silently skipped for diagnostic emission but still contribute
// use sites to cross-file analysis.
#[test]
fn test_generated_file_skipped_but_contributes_symbols() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("DESCRIPTION", ""),
        ("NAMESPACE", ""),
        ("R/foo2.R", "f <- function() 1 + 1\n"),
        ("R/foo.R", "# Generated by foo\nany(is.na(x))\nf()\n"),
    ])?;

    // any(is.na(x)) isn't reported because it's in generated file.
    // f() isn't reported because it's used in generated file.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("ALL")
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
fn test_must_start_with_generated_by_to_be_ignored() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("DESCRIPTION", ""),
        ("NAMESPACE", ""),
        ("R/foo2.R", "f <- function() 1 + 1\n"),
        (
            "R/foo.R",
            "# This is not generated by foo\nany(is.na(x))\nf()\n",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("ALL")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/foo.R:2:1
      |
    2 | any(is.na(x))
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
