use crate::helpers::{CliTest, CommandExt};

#[test]
fn test_jarl_ignore_inline_suppression() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.R",
        "
# jarl-ignore any_is_na: legacy code
any(is.na(x))
",
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

#[test]
fn test_jarl_ignore_inline_suppression_in_pipe() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.R",
        "
# jarl-ignore any_is_na: legacy code
x |>
  is.na() |>
  any()
",
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

#[test]
fn test_jarl_ignore_file_suppression() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.R",
        "# jarl-ignore-file any_is_na: this file has many false positives
any(is.na(x))
any(is.na(y))
any(is.na(z))
",
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

#[test]
fn test_jarl_ignore_region_suppression() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.R",
        "
any(is.na(x))

# jarl-ignore-start any_is_na: debugging section
any(is.na(y))
any(is.na(z))
# jarl-ignore-end any_is_na

any(is.na(w))
",
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
    warning: any_is_na
     --> test.R:2:1
      |
    2 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_is_na
     --> test.R:9:1
      |
    9 | any(is.na(w))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.
    2 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_jarl_ignore_cascading_suppression() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.R",
        "
# jarl-ignore any_is_na: cascades to children
x <- function(x) {
    any(is.na(y))
}
any(is.na(y))
",
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
    warning: any_is_na
     --> test.R:6:1
      |
    6 | any(is.na(y))
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
fn test_jarl_ignore_multiple_rules_with_extend_select() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.R",
        "
# jarl-ignore any_is_na: first rule
# jarl-ignore assignment: second rule
x = any(is.na(y))
x
",
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--extend-select")
            .arg("assignment")
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
fn test_jarl_ignore_nested_in_call_second_argument() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.R",
        "
foo(
  first_arg,
  # jarl-ignore implicit_assignment: suppressing second arg
  x <- 1
)
x
",
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

#[test]
fn test_nolint_format_not_recognized() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.R",
        "
# nolint
any(is.na(x))
# nolint: any_is_na
any(is.na(y))
# nolint start
any(is.na(z))
# nolint end
",
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
    warning: any_is_na
     --> test.R:3:1
      |
    3 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_is_na
     --> test.R:5:1
      |
    5 | any(is.na(y))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_is_na
     --> test.R:7:1
      |
    7 | any(is.na(z))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 3 errors.
    3 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_fix_skips_internal_comments_with_outer_comments_460() -> anyhow::Result<()> {
    let original = "# leading comment\n!(x \n # hello there \n >= y)\n";
    let case = CliTest::with_file("test.R", original)?;

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

    success: false
    exit_code: 1
    ----- stdout -----
    warning: comparison_negation
     --> test.R:2:1
      |
    2 | / !(x 
    3 | |  # hello there 
    4 | |  >= y)
      | |______- `!(x >= y)` can be simplified.
      |
      = help: Use `x < y` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    let fixed = case.read_file("test.R")?;
    assert_eq!(fixed, original);

    Ok(())
}
