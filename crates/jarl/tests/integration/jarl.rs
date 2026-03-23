use crate::helpers::{CliTest, CommandExt};

#[test]
fn test_must_pass_path() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .run()
            .normalize_os_executable_name(),
        @r#"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Check a set of files or directories

    Usage: jarl check [OPTIONS] <FILES>...

    Arguments:
      <FILES>...  List of files or directories to check or fix lints, for example `jarl check .`.

    Options:
      -f, --fix                            Automatically fix issues detected by the linter.
      -u, --unsafe-fixes                   Include fixes that may not retain the original intent of the  code.
          --fix-only                       Apply fixes to resolve lint violations, but don't report on leftover violations. Implies `--fix`.
          --allow-dirty                    Apply fixes even if the Git branch is not clean, meaning that there are uncommitted files.
          --allow-no-vcs                   Apply fixes even if there is no version control system.
      -s, --select <SELECT>                Names of rules to include, separated by a comma (no spaces). This also accepts names of groups of rules, such as "PERF". [default: ""]
      -e, --extend-select <EXTEND_SELECT>  Like `--select` but adds additional rules in addition to those already specified. [default: ""]
      -i, --ignore <IGNORE>                Names of rules to exclude, separated by a comma (no spaces). This also accepts names of groups of rules, such as "PERF". [default: ""]
      -w, --with-timing                    Show the time taken by the function.
      -m, --min-r-version <MIN_R_VERSION>  The mimimum R version to be used by the linter. Some rules only work starting from a specific version.
          --output-format <OUTPUT_FORMAT>  Output serialization format for violations. [default: full] [possible values: full, concise, github, json]
          --assignment <ASSIGNMENT>        [DEPRECATED: use `[lint.assignment]` in jarl.toml] Assignment operator to use, can be either `<-` or `=`.
          --no-default-exclude             Do not apply the default set of file patterns that should be excluded.
          --statistics                     Show counts for every rule with at least one violation.
          --add-jarl-ignore[=<REASON>]     Automatically insert a `# jarl-ignore` comment to suppress all violations.
                                           The default reason can be customized with `--add-jarl-ignore="my_reason"`.
      -h, --help                           Print help (see more with '--help')

    Global options:
          --log-level <LOG_LEVEL>  The log level. One of: `error`, `warn`, `info`, `debug`, or `trace`. Defaults to `warn`
    "#
    );

    Ok(())
}

#[test]
fn test_no_r_files() -> anyhow::Result<()> {
    let case = CliTest::new()?;
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
    Warning: No R files found under the given path(s).

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_parsing_error() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "f <-")?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    Error: Failed to parse test.R due to syntax errors.
    "
    );

    Ok(())
}

#[test]
fn test_parsing_error_for_some_files() -> anyhow::Result<()> {
    let case = CliTest::with_files([("test.R", "f <-"), ("test2.R", "any(is.na(x))")])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 255
    ----- stdout -----
    warning: any_is_na
     --> test2.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    Error: Failed to parse test.R due to syntax errors.
    "
    );

    Ok(())
}

#[test]
fn test_parsing_weird_raw_strings() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "c(r\"(abc(\\w+))\")\nr\"(c(\"\\dots\"))\"")?;
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
fn test_parsing_braced_anonymous_function() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "{ a }(10)")?;
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
fn test_no_lints() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "any(x)")?;
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
fn test_one_lint() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "any(is.na(x))")?;
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
     --> test.R:1:1
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
fn test_several_lints_one_file() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "any(is.na(x))\nany(duplicated(x))")?;

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
     --> test.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_duplicated
     --> test.R:2:1
      |
    2 | any(duplicated(x))
      | ------------------ `any(duplicated(...))` is inefficient.
      |
      = help: Use `anyDuplicated(...) > 0` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.
    2 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_several_lints_several_files() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("test.R", "any(is.na(x))"),
        ("test2.R", "any(duplicated(x))"),
    ])?;

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
     --> test.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_duplicated
     --> test2.R:1:1
      |
    1 | any(duplicated(x))
      | ------------------ `any(duplicated(...))` is inefficient.
      |
      = help: Use `anyDuplicated(...) > 0` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.
    2 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_not_all_fixable_lints() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("test.R", "any(is.na(x))"),
        ("test2.R", "list(x = 1, x = 2)"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @r#"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> test.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: duplicated_arguments
     --> test2.R:1:1
      |
    1 | list(x = 1, x = 2)
      | ------------------ Avoid duplicated arguments in function calls. Duplicated argument(s): "x".
      |


    ── Summary ──────────────────────────────────────
    Found 2 errors.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "#
    );

    Ok(())
}

#[test]
fn test_corner_case() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "x %>% length()")?;
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
fn test_fix_options() -> anyhow::Result<()> {
    // File with 3 lints:
    // - any_is_na (has fix)
    // - class_equals (has unsafe fix)
    // - duplicated_arguments (has no fix)
    let case = CliTest::with_file(
        "test.R",
        "any(is.na(x))\nclass(x) == 'foo'\nlist(x = 1, x = 2)",
    )?;
    let test_contents = "any(is.na(x))\nclass(x) == 'foo'\nlist(x = 1, x = 2)";

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--fix")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name(),
        @r#"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: duplicated_arguments
     --> test.R:3:1
      |
    3 | list(x = 1, x = 2)
      | ------------------ Avoid duplicated arguments in function calls. Duplicated argument(s): "x".
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "#
    );

    case.write_file("test.R", test_contents)?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--fix")
            .arg("--unsafe-fixes")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name(),
        @r#"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: duplicated_arguments
     --> test.R:3:1
      |
    3 | list(x = 1, x = 2)
      | ------------------ Avoid duplicated arguments in function calls. Duplicated argument(s): "x".
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "#
    );

    case.write_file("test.R", test_contents)?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--fix")
            .arg("--unsafe-fixes")
            .arg("--fix-only")
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

    case.write_file("test.R", test_contents)?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--fix")
            .arg("--fix-only")
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

    case.write_file("test.R", test_contents)?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--unsafe-fixes")
            .arg("--fix-only")
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
fn test_safe_and_unsafe_lints() -> anyhow::Result<()> {
    let case = CliTest::with_files([("test.R", "any(is.na(x))"), ("test2.R", "!all.equal(x, y)")])?;

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
     --> test.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: all_equal
     --> test2.R:1:1
      |
    1 | !all.equal(x, y)
      | ---------------- If `all.equal()` is false, it will return a string and not `FALSE`.
      |
      = help: Wrap `all.equal()` in `isTRUE()`, or replace it by `identical()` if no tolerance is required.


    ── Summary ──────────────────────────────────────
    Found 2 errors.
    1 fixable with the `--fix` option (1 hidden fix can be enabled with the `--unsafe-fixes` option).

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_newline_character_in_string() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "print(\"hi there\\n\")\nany(is.na(x))")?;

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
     --> test.R:2:1
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

// ---------------------------------------------------------------------------
// Overlapping fixes across rules don't corrupt the file
// ---------------------------------------------------------------------------

/// When `which_grepl` and `fixed_regex` both fire on the same call, their fixes
/// overlap. The overlap detection must correctly skip the inner fix on every
/// iteration, even when accumulated length changes shift positions. With a
/// buggy offset tracker this broke starting at 5 overlapping pairs.
///
/// This specific example used to make the `fix-check` workflow fail.
#[test]
fn test_overlapping_fixes_no_corruption() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.R",
        r#"f <- function() {
  expect_length(which(grepl("A__", str)), 1L)
  expect_length(which(grepl("B__", str)), 1L)
  expect_length(which(grepl("C__", str)), 0L)
  expect_length(which(grepl("D__", str)), 0L)
  expect_length(which(grepl("E__", str)), 1L)
}
"#,
    )?;

    case.command()
        .arg("check")
        .arg(".")
        .arg("--select")
        .arg("ALL")
        .arg("--fix")
        .arg("--allow-no-vcs")
        .run();

    let fixed = case.read_file("test.R")?;
    insta::assert_snapshot!(
        fixed,
        @r#"
    f <- function() {
      expect_length(grep("A__", str, fixed = TRUE), 1L)
      expect_length(grep("B__", str, fixed = TRUE), 1L)
      expect_length(grep("C__", str, fixed = TRUE), 0L)
      expect_length(grep("D__", str, fixed = TRUE), 0L)
      expect_length(grep("E__", str, fixed = TRUE), 1L)
    }
    "#
    );

    Ok(())
}
