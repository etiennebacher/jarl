use std::process::Command;

use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_one_non_existing_selected_rule() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("foo")
            .run()
            .normalize_os_executable_name(),
        @"
success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: Unknown rules in `--select`: foo
"
    );

    Ok(())
}

#[test]
fn test_several_non_existing_selected_rules() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("foo,any_is_na,barbaz")
            .run()
            .normalize_os_executable_name(),
        @"
success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: Unknown rules in `--select`: foo, barbaz
"
    );

    Ok(())
}

#[test]
fn test_one_non_existing_ignored_rule() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--ignore")
            .arg("foo")
            .run()
            .normalize_os_executable_name(),
        @"
success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: Unknown rules in `--ignore`: foo
"
    );

    Ok(())
}

#[test]
fn test_several_non_existing_ignored_rules() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--ignore")
            .arg("foo,any_is_na,barbaz")
            .run()
            .normalize_os_executable_name(),
        @"
success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: Unknown rules in `--ignore`: foo, barbaz
"
    );

    Ok(())
}

#[test]
fn test_selected_and_ignored() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("any_is_na")
            .arg("--ignore")
            .arg("any_is_na")
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

#[test]
fn test_correct_rule_selection_and_exclusion() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    let test_path_2 = "test2.R";
    let test_contents_2 = "any(duplicated(x))";
    std::fs::write(directory.join(test_path_2), test_contents_2)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("any_is_na")
            .arg("--ignore")
            .arg("any_duplicated")
            .run()
            .normalize_os_executable_name(),
        @r"
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
fn test_select_rule_group() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
any(is.na(x))
!all.equal(x, y)
";
    std::fs::write(directory.join(test_path), test_contents)?;

    // Works with only group name
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("SUSP")
            .run()
            .normalize_os_executable_name(),
        @r"
    success: false
    exit_code: 1
    ----- stdout -----
    warning: all_equal
     --> test.R:3:1
      |
    3 | !all.equal(x, y)
      | ---------------- If `all.equal()` is false, it will return a string and not `FALSE`.
      |
      = help: Wrap `all.equal()` in `isTRUE()`, or replace it by `identical()` if no tolerance is required.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fix is available with the `--fix --unsafe-fixes` option.

    ----- stderr -----
    "
    );

    // Can mix group name and rule name
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("any_is_na,SUSP")
            .run()
            .normalize_os_executable_name(),
        @r"
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

    warning: all_equal
     --> test.R:3:1
      |
    3 | !all.equal(x, y)
      | ---------------- If `all.equal()` is false, it will return a string and not `FALSE`.
      |
      = help: Wrap `all.equal()` in `isTRUE()`, or replace it by `identical()` if no tolerance is required.


    ── Summary ──────────────────────────────────────
    Found 2 errors.
    1 fixable with the `--fix` option (1 hidden fix can be enabled with the `--unsafe-fixes` option).

    ----- stderr -----
    "
    );

    // Can mix group name and rule name that is part of the same group
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("all_equal,SUSP")
            .run()
            .normalize_os_executable_name(),
        @r"
    success: false
    exit_code: 1
    ----- stdout -----
    warning: all_equal
     --> test.R:3:1
      |
    3 | !all.equal(x, y)
      | ---------------- If `all.equal()` is false, it will return a string and not `FALSE`.
      |
      = help: Wrap `all.equal()` in `isTRUE()`, or replace it by `identical()` if no tolerance is required.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fix is available with the `--fix --unsafe-fixes` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_ignore_rule_group() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
any(is.na(x))
!all.equal(x, y)
";
    std::fs::write(directory.join(test_path), test_contents)?;

    // Works with only group name
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--ignore")
            .arg("SUSP")
            .run()
            .normalize_os_executable_name(),
        @r"
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

    // Can mix group name and rule name
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--ignore")
            .arg("any_is_na,SUSP")
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

    // Can mix group name and rule name that is part of the same group
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--ignore")
            .arg("all_equal,SUSP")
            .run()
            .normalize_os_executable_name(),
        @r"
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

#[test]
fn test_invalid_rule_group() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    // Works with only group name
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--ignore")
            .arg("FOOBAR,SUSP")
            .run()
            .normalize_os_executable_name(),
        @"
success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: Unknown rules in `--ignore`: FOOBAR
"
    );

    Ok(())
}

#[test]
fn test_select_ignore_interaction_with_rule_group() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
any(is.na(x))
!all.equal(x, y)
";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("SUSP")
            .arg("--ignore")
            .arg("SUSP")
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

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("SUSP")
            .arg("--ignore")
            .arg("PERF")
            .run()
            .normalize_os_executable_name(),
        @r"
    success: false
    exit_code: 1
    ----- stdout -----
    warning: all_equal
     --> test.R:3:1
      |
    3 | !all.equal(x, y)
      | ---------------- If `all.equal()` is false, it will return a string and not `FALSE`.
      |
      = help: Wrap `all.equal()` in `isTRUE()`, or replace it by `identical()` if no tolerance is required.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fix is available with the `--fix --unsafe-fixes` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_non_default_rule_groups_are_ignored() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
any(is.na(x))
expect_equal(foo(x), TRUE)
";
    std::fs::write(directory.join(test_path), test_contents)?;

    // The rule group TESTTHAT is disabled by default, so the second line is not
    // reported.
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @r"
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

#[test]
fn test_select_all_keyword() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
any(is.na(x))
expect_equal(foo(x), TRUE)
";
    std::fs::write(directory.join(test_path), test_contents)?;

    // Using ALL should select all rules including opt-in ones like TESTTHAT
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("ALL")
            .run()
            .normalize_os_executable_name(),
        @r"
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

    warning: expect_true_false
     --> test.R:3:1
      |
    3 | expect_equal(foo(x), TRUE)
      | -------------------------- `expect_equal(x, TRUE)` is not as clear as `expect_true(x)`.
      |
      = help: Use `expect_true(x)` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.
    2 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    // ALL can be combined with ignore
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("ALL")
            .arg("--ignore")
            .arg("TESTTHAT")
            .run()
            .normalize_os_executable_name(),
        @r"
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

#[test]
fn test_extend_select() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
any(is.na(x))
expect_equal(foo(x), TRUE)
";
    std::fs::write(directory.join(test_path), test_contents)?;

    // With extend-select TESTTHAT, both default rules and TESTTHAT rules are active
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--extend-select")
            .arg("TESTTHAT")
            .run()
            .normalize_os_executable_name(),
        @r"
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

    warning: expect_true_false
     --> test.R:3:1
      |
    3 | expect_equal(foo(x), TRUE)
      | -------------------------- `expect_equal(x, TRUE)` is not as clear as `expect_true(x)`.
      |
      = help: Use `expect_true(x)` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.
    2 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    // extend-select can also be used with specific rule names
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--extend-select")
            .arg("expect_true_false")
            .run()
            .normalize_os_executable_name(),
        @r"
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

    warning: expect_true_false
     --> test.R:3:1
      |
    3 | expect_equal(foo(x), TRUE)
      | -------------------------- `expect_equal(x, TRUE)` is not as clear as `expect_true(x)`.
      |
      = help: Use `expect_true(x)` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.
    2 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_extend_select_unknown_rule() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--extend-select")
            .arg("FOO")
            .run()
            .normalize_os_executable_name(),
        @"
success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: Unknown rules in `--extend-select`: FOO
"
    );
    Ok(())
}

#[test]
fn test_deprecated_rule_warning_from_cli() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "browser()";
    std::fs::write(directory.join(test_path), test_contents)?;

    // Selecting `browser` via --select should emit a deprecation warning on stderr
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("browser")
            .run()
            .normalize_os_executable_name(),
        @r"
    success: false
    exit_code: 1
    ----- stdout -----
    warning: browser
     --> test.R:1:1
      |
    1 | browser()
      | --------- Calls to `browser()` should be removed.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ── Warnings ─────────────────────────────────────
    Rule `browser` is deprecated since v0.5.0. Use `undesirable_function` instead.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_deprecated_rule_warning_from_toml() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "browser()";
    std::fs::write(directory.join(test_path), test_contents)?;

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
select = ["browser"]
"#,
    )?;

    // Using `browser` in TOML select should emit a deprecation warning
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @r"
    success: false
    exit_code: 1
    ----- stdout -----
    warning: browser
     --> test.R:1:1
      |
    1 | browser()
      | --------- Calls to `browser()` should be removed.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ── Warnings ─────────────────────────────────────
    Rule `browser` is deprecated since v0.5.0. Use `undesirable_function` instead.

    ----- stderr -----
    "
    );

    Ok(())
}
