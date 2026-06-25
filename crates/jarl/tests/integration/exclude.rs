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

#[test]
fn test_exclude_cli_arg_no_value() -> anyhow::Result<()> {
    let case = CliTest::with_files([("foo.R", "x = 1\n")])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--exclude")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: equal sign is needed when assigning values to '--exclude=<FILES>'

    Usage: jarl check [OPTIONS] <FILES>...

    For more information, try '--help'.
    "
    );

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--exclude")
            .arg("foo.R")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: equal sign is needed when assigning values to '--exclude=<FILES>'

    Usage: jarl check [OPTIONS] <FILES>...

    For more information, try '--help'.
    "
    );

    Ok(())
}

/// The `--exclude` CLI flag skips the matched file's own diagnostics
#[test]
fn test_cli_exclude_skips_file() -> anyhow::Result<()> {
    let case = CliTest::with_files([("good.R", "x <- 1\n"), ("bad.R", "y = 2\n")])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .arg("--exclude=bad.R")
            .run()
            .normalize_os_executable_name(),
        @r"

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

/// `--exclude` accepts a comma-separated list of patterns.
#[test]
fn test_cli_exclude_comma_separated() -> anyhow::Result<()> {
    let case = CliTest::with_files([("a.R", "a = 1\n"), ("b.R", "b = 2\n"), ("c.R", "c = 3\n")])?;

    // a.R and b.R are excluded, only c.R is reported.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .arg("--exclude=a.R,b.R")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: assignment
     --> c.R:1:1
      |
    1 | c = 3
      | --- Use `<-` for assignment.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_cli_exclude_glob_function_def_524() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("DESCRIPTION", ""),
        ("R/foo.R", "f <- function() {}\n"),
        ("R/bar.R", "f <- function() {}\n"),
        ("R/baz.R", "f <- function() {}\n"),
    ])?;

    // Do not exclude any file -> report duplicated definitions
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("R/")
            .arg("--select")
            .arg("duplicated_function_definition")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: duplicated_function_definition
     --> R/baz.R:1:1
      |
    1 | f <- function() {}
      | - `f` is defined more than once in this package.
      |
      = help: Other definition at R/bar.R:1:1

    warning: duplicated_function_definition
     --> R/foo.R:1:1
      |
    1 | f <- function() {}
      | - `f` is defined more than once in this package.
      |
      = help: Other definition at R/bar.R:1:1


    ── Summary ──────────────────────────────────────
    Found 2 errors.

    ----- stderr -----
    "
    );

    // Files that contain one of the duplicated definitions are explicitly excluded
    // but still devtools::load_all() would load several function definitions with
    // the same name and we want to flag that.
    // Might give less control to the user so can be reconsidered in the future.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("R/")
            .arg("--exclude=R/b*.R")
            .arg("--select")
            .arg("duplicated_function_definition")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: duplicated_function_definition
     --> R/foo.R:1:1
      |
    1 | f <- function() {}
      | - `f` is defined more than once in this package.
      |
      = help: Other definition at R/bar.R:1:1


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

/// `--exclude` accepts glob patterns, including path-anchored globs (those
/// containing a `/`), even when no `jarl.toml` is present.
#[test]
fn test_cli_exclude_glob_patterns() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("keep.R", "a = 1\n"),
        ("R/gen.R", "b = 2\n"),
        ("R/keep.R", "c <- 3\n"),
    ])?;

    // `R/*.R` is anchored to the run directory and must match `R/gen.R` only.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .arg("--exclude=R/*.R")
            .run()
            .normalize_os_executable_name(),
        @r"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: assignment
     --> keep.R:1:1
      |
    1 | a = 1
      | --- Use `<-` for assignment.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_cli_exclude_overrides_hardcoded_path_passed_in_files() -> anyhow::Result<()> {
    let case = CliTest::with_files([("R/gen.R", "b = 2\n"), ("R/keep.R", "c = 3\n")])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("R")
            .arg("--exclude=R/g*.R")
            .arg("--select")
            .arg("assignment")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: assignment
     --> R/keep.R:1:1
      |
    1 | c = 3
      | --- Use `<-` for assignment.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_cli_exclude_wrong_glob_patterns() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("keep.R", "a = 1\n"),
        ("R/gen.R", "b = 2\n"),
        ("R/keep.R", "c <- 3\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--exclude=[*.R")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: invalid `--exclude` pattern: error parsing glob '[*.R': unclosed character class; missing ']'
    "
    );

    Ok(())
}

/// A file excluded via `--exclude` should still contribute symbol usages for
/// cross-file analysis, mirroring the behavior of `exclude` in `jarl.toml`.
#[test]
fn test_cli_excluded_file_contributes_symbols() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("DESCRIPTION", ""),
        ("NAMESPACE", ""),
        ("R/foo2.R", "f <- function() 1 + 1\n"),
        ("R/foo.R", "f()\n"),
    ])?;

    // f() should NOT be reported as unused because excluded foo.R calls it.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_function")
            .arg("--exclude=R/foo.R")
            .run()
            .normalize_os_executable_name(),
        @r"

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

/// A file excluded via `--exclude` should still contribute assignments for
/// cross-file analysis, mirroring the behavior of `exclude` in `jarl.toml`.
#[test]
fn test_cli_excluded_file_contributes_assignments() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("DESCRIPTION", ""),
        ("NAMESPACE", ""),
        ("R/foo.R", "f <- function() 1\n"),
        ("R/foo2.R", "f <- function() 2\n"),
    ])?;

    // foo2.R should report f as duplicated (other definition in excluded foo.R).
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("duplicated_function_definition")
            .arg("--exclude=R/foo.R")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @r"

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
