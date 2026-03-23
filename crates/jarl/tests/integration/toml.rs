use crate::helpers::{CliTest, CommandExt};

#[test]
fn test_empty_toml_uses_all_rules() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
        (
            "jarl.toml",
            r#"
[lint]
"#,
        ),
    ])?;

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
fn test_empty_select_array() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
select = []
"#,
        ),
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    case.write_file(
        "jarl.toml",
        r#"
[lint]
select = [""]
"#,
    )?;

    case.write_file("test.R", "any(is.na(x))\nany(duplicated(x))")?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @r#"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Unknown rules in field `select` in 'jarl.toml': "" (empty or whitespace-only not allowed)
    "#
    );

    Ok(())
}

#[test]
fn test_empty_ignore_array() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
ignore = []
"#,
        ),
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
    ])?;

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

    case.write_file(
        "jarl.toml",
        r#"
[lint]
ignore = [""]
"#,
    )?;

    case.write_file("test.R", "any(is.na(x))\nany(duplicated(x))")?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @r#"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Unknown rules in field `ignore` in 'jarl.toml': "" (empty or whitespace-only not allowed)
    "#
    );

    Ok(())
}

#[test]
fn test_toml_select() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na"]
"#,
        ),
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
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


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_toml_select_with_group() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na", "SUSP"]
"#,
        ),
        (
            "test.R",
            "
any(is.na(x))
any(duplicated(x))
!all.equal(x, y)
",
        ),
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
     --> test.R:2:1
      |
    2 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: all_equal
     --> test.R:4:1
      |
    4 | !all.equal(x, y)
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
fn test_toml_ignore() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
ignore = ["any_duplicated"]
"#,
        ),
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
    ])?;

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
fn test_toml_select_and_ignore() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na", "any_duplicated", "length_levels"]
ignore = ["length_levels"]
"#,
        ),
        (
            "test.R",
            r#"any(is.na(x))
any(duplicated(x))
length(levels(x))"#,
        ),
    ])?;

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
fn test_cli_select_overrides_toml() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na"]
ignore = ["length_levels"]
"#,
        ),
        (
            "test.R",
            r#"any(is.na(x))
any(duplicated(x))
length(levels(x))"#,
        ),
    ])?;

    // CLI select should override TOML select, but TOML ignore should still apply
    // TODO: not sure this is correct, length_levels is ignored but since it's
    // put explicitly in the CLI maybe it should raise?
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("any_duplicated,length_levels")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_duplicated
     --> test.R:2:1
      |
    2 | any(duplicated(x))
      | ------------------ `any(duplicated(...))` is inefficient.
      |
      = help: Use `anyDuplicated(...) > 0` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_cli_ignore_adds_to_toml() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na", "any_duplicated", "length_levels"]
ignore = ["length_levels"]
"#,
        ),
        (
            "test.R",
            r#"any(is.na(x))
any(duplicated(x))
length(levels(x))"#,
        ),
    ])?;

    // CLI ignore should add to TOML ignore, using TOML select
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--ignore")
            .arg("any_is_na")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_duplicated
     --> test.R:2:1
      |
    2 | any(duplicated(x))
      | ------------------ `any(duplicated(...))` is inefficient.
      |
      = help: Use `anyDuplicated(...) > 0` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_cli_overrides_toml_completely() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na"]
ignore = ["any_duplicated"]
"#,
        ),
        (
            "test.R",
            r#"any(is.na(x))
any(duplicated(x))
length(levels(x))"#,
        ),
    ])?;

    // Both CLI select and ignore should completely override TOML
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("length_levels,any_duplicated")
            .arg("--ignore")
            .arg("length_levels")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
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
fn test_invalid_toml_select_rule() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na", "foo"]
"#,
        ),
        ("test.R", "any(is.na(x))"),
    ])?;

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
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Unknown rules in field `select` in 'jarl.toml': foo
    "
    );

    Ok(())
}

#[test]
fn test_invalid_toml_ignore_rule() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
ignore = ["foo", "bar"]
"#,
        ),
        ("test.R", "any(is.na(x))"),
    ])?;

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
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Unknown rules in field `ignore` in 'jarl.toml': foo, bar
    "
    );

    Ok(())
}

#[test]
fn test_malformed_toml_syntax() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint
select = ["any_is_na"
"#,
        ),
        ("test.R", "any(is.na(x))"),
    ])?;

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
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Failed to parse [TEMP_DIR]/jarl.toml:
    TOML parse error at line 2, column 6
      |
    2 | [lint
      |      ^
    invalid table header
    expected `.`, `]`
    "
    );

    Ok(())
}

#[test]
fn test_unknown_toml_field() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na"]
unknown_field = ["value"]
"#,
        ),
        ("test.R", "any(is.na(x))"),
    ])?;

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
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Invalid configuration in [TEMP_DIR]/jarl.toml:
    Unknown field `unknown_field` in `[lint]`. Expected one of: `select`, `extend-select`, `ignore`, `fixable`, `unfixable`, `exclude`, `default-exclude`, `include`, `check-roxygen`, `fix-roxygen`.
    "
    );

    Ok(())
}

#[test]
fn test_toml_without_linter_section() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
# Just a comment, no linter section
"#,
        ),
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
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
fn test_empty_string_in_toml_ignore() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
ignore = ["any_duplicated", "", "any_is_na"]
"#,
        ),
        ("test.R", "any(is.na(x))"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @r#"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Unknown rules in field `ignore` in 'jarl.toml': "" (empty or whitespace-only not allowed)
    "#
    );

    Ok(())
}

#[test]
fn test_whitespace_only_in_toml_select() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na", "   ", "any_duplicated"]
"#,
        ),
        ("test.R", "any(is.na(x))"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @r#"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Unknown rules in field `select` in 'jarl.toml': "" (empty or whitespace-only not allowed)
    "#
    );

    Ok(())
}

#[test]
fn test_no_toml_file_uses_all_rules() -> anyhow::Result<()> {
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
fn test_default_exclude_works() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
"#,
        ),
        ("cpp11.R", "any(is.na(x))\nany(duplicated(x))"),
    ])?;

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

    // "default-exclude" specified by the user
    case.write_file(
        "jarl.toml",
        r#"
[lint]
default-exclude = false
"#,
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
     --> cpp11.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_duplicated
     --> cpp11.R:2:1
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
fn test_default_exclude_wrong_values() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "jarl.toml",
        r#"
[lint]
default-exclude = 1
"#,
    )?;

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
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Failed to parse [TEMP_DIR]/jarl.toml:
    TOML parse error at line 3, column 19
      |
    3 | default-exclude = 1
      |                   ^
    invalid type: integer `1`, expected a boolean
    "
    );

    // "default-exclude" specified by the user
    case.write_file(
        "jarl.toml",
        r#"
[lint]
default-exclude = ["a"]
"#,
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @r#"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Failed to parse [TEMP_DIR]/jarl.toml:
    TOML parse error at line 3, column 19
      |
    3 | default-exclude = ["a"]
      |                   ^^^^^
    invalid type: sequence, expected a boolean
    "#
    );

    Ok(())
}

#[test]
fn test_exclude_single_file() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
exclude = ["excluded.R"]
"#,
        ),
        ("excluded.R", "any(is.na(x))"),
        ("included.R", "any(is.na(y))"),
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
     --> included.R:1:1
      |
    1 | any(is.na(y))
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
fn test_exclude_directory() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
exclude = ["excluded_dir/"]
"#,
        ),
        ("excluded_dir/file.R", "any(is.na(x))"),
        ("included.R", "any(is.na(y))"),
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
     --> included.R:1:1
      |
    1 | any(is.na(y))
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
fn test_exclude_glob_pattern() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
exclude = ["test-*.R"]
"#,
        ),
        ("test-one.R", "any(is.na(x))"),
        ("test-two.R", "any(is.na(y))"),
        ("normal.R", "any(is.na(z))"),
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
     --> normal.R:1:1
      |
    1 | any(is.na(z))
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
fn test_exclude_multiple_patterns() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
exclude = ["excluded.R", "temp/", "*.tmp.R"]
"#,
        ),
        ("excluded.R", "any(is.na(a))"),
        ("temp/file.R", "any(is.na(b))"),
        ("test.tmp.R", "any(is.na(c))"),
        ("included.R", "any(is.na(d))"),
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
     --> included.R:1:1
      |
    1 | any(is.na(d))
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
fn test_exclude_with_default_exclude_false() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
default-exclude = false
exclude = ["custom_exclude.R"]
"#,
        ),
        ("cpp11.R", "any(is.na(x))"),
        ("custom_exclude.R", "any(is.na(y))"),
        ("normal.R", "any(is.na(z))"),
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
     --> cpp11.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_is_na
     --> normal.R:1:1
      |
    1 | any(is.na(z))
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
fn test_exclude_nested_directory_pattern() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
exclude = ["**/test/**"]
"#,
        ),
        ("src/test/file.R", "any(is.na(x))"),
        ("lib/test/deep/file.R", "any(is.na(y))"),
        ("other/main.R", "any(is.na(z))"),
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
     --> other/main.R:1:1
      |
    1 | any(is.na(z))
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
fn test_exclude_empty_array() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
exclude = []
"#,
        ),
        ("test.R", "any(is.na(x))"),
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


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_exclude_wrong_values() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "jarl.toml",
        r#"
[lint]
exclude = true
"#,
    )?;

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
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Failed to parse [TEMP_DIR]/jarl.toml:
    TOML parse error at line 3, column 11
      |
    3 | exclude = true
      |           ^^^^
    invalid type: boolean `true`, expected a sequence
    "
    );

    case.write_file(
        "jarl.toml",
        r#"
[lint]
exclude = 1
"#,
    )?;

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
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Failed to parse [TEMP_DIR]/jarl.toml:
    TOML parse error at line 3, column 11
      |
    3 | exclude = 1
      |           ^
    invalid type: integer `1`, expected a sequence
    "
    );

    case.write_file(
        "jarl.toml",
        r#"
[lint]
exclude = ["a", 1]
"#,
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @r#"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Failed to parse [TEMP_DIR]/jarl.toml:
    TOML parse error at line 3, column 17
      |
    3 | exclude = ["a", 1]
      |                 ^
    invalid type: integer `1`, expected a string
    "#
    );

    Ok(())
}

#[test]
fn test_toml_fixable_basic() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
fixable = ["any_is_na"]
"#,
        ),
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
    ])?;

    // Keep the snapshot to show that the unfixable violation is still reported.
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
    warning: any_duplicated
     --> test.R:2:1
      |
    2 | any(duplicated(x))
      | ------------------ `any(duplicated(...))` is inefficient.
      |
      = help: Use `anyDuplicated(...) > 0` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    // Only any_is_na should be fixed
    let fixed_contents = case.read_file("test.R")?;
    insta::assert_snapshot!(fixed_contents,
        @"
    anyNA(x)
    any(duplicated(x))
    "
    );

    Ok(())
}

#[test]
fn test_toml_unfixable_basic() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
unfixable = ["any_is_na"]
"#,
        ),
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
    ])?;

    let _ = &mut case
        .command()
        .arg("check")
        .arg(".")
        .arg("--fix")
        .arg("--allow-no-vcs")
        .run()
        .normalize_os_executable_name();

    // Only any_duplicated should be fixed
    let fixed_contents = case.read_file("test.R")?;
    insta::assert_snapshot!(fixed_contents,
        @"
    any(is.na(x))
    anyDuplicated(x) > 0
    "
    );

    Ok(())
}

#[test]
fn test_toml_fixable_with_group() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
fixable = ["PERF"]
"#,
        ),
        (
            "test.R",
            "any(is.na(x))\nany(duplicated(x))\nlength(levels(x))",
        ),
    ])?;

    let _ = &mut case
        .command()
        .arg("check")
        .arg(".")
        .arg("--fix")
        .arg("--allow-no-vcs")
        .run()
        .normalize_os_executable_name();

    // Only PERF rules should be fixed
    let fixed_contents = case.read_file("test.R")?;
    insta::assert_snapshot!(fixed_contents,
        @"
    anyNA(x)
    anyDuplicated(x) > 0
    length(levels(x))
    "
    );

    Ok(())
}

#[test]
fn test_toml_unfixable_with_group() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
unfixable = ["PERF"]
"#,
        ),
        (
            "test.R",
            "any(is.na(x))\nany(duplicated(x))\nlength(levels(x))",
        ),
    ])?;

    let _ = &mut case
        .command()
        .arg("check")
        .arg(".")
        .arg("--fix")
        .arg("--allow-no-vcs")
        .run()
        .normalize_os_executable_name();

    // PERF rules should not be fixed
    let fixed_contents = case.read_file("test.R")?;
    insta::assert_snapshot!(fixed_contents,
        @"
    any(is.na(x))
    any(duplicated(x))
    nlevels(x)
    "
    );

    Ok(())
}

#[test]
fn test_toml_fixable_and_unfixable_conflict() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
fixable = ["any_is_na", "any_duplicated"]
unfixable = ["any_is_na"]
"#,
        ),
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
    ])?;

    let _ = &mut case
        .command()
        .arg("check")
        .arg(".")
        .arg("--fix")
        .arg("--allow-no-vcs")
        .run()
        .normalize_os_executable_name();

    // any_is_na should not be fixed
    let fixed_contents = case.read_file("test.R")?;
    insta::assert_snapshot!(fixed_contents,
        @"
    any(is.na(x))
    anyDuplicated(x) > 0
    "
    );

    Ok(())
}

#[test]
fn test_toml_unnecessary_unfixable() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
fixable = ["any_is_na"]
unfixable = ["any_duplicated"]
"#,
        ),
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
    ])?;

    let _ = &mut case
        .command()
        .arg("check")
        .arg(".")
        .arg("--fix")
        .arg("--allow-no-vcs")
        .run()
        .normalize_os_executable_name();

    // any_is_na should not be fixed
    let fixed_contents = case.read_file("test.R")?;
    insta::assert_snapshot!(fixed_contents,
        @"
    anyNA(x)
    any(duplicated(x))
    "
    );

    Ok(())
}

#[test]
fn test_toml_fixable_empty_array() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
fixable = []
"#,
        ),
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
    ])?;

    let _ = &mut case
        .command()
        .arg("check")
        .arg(".")
        .arg("--fix")
        .arg("--allow-no-vcs")
        .run()
        .normalize_os_executable_name();

    let fixed_contents = case.read_file("test.R")?;
    insta::assert_snapshot!(fixed_contents,
        @"
    any(is.na(x))
    any(duplicated(x))
    "
    );

    Ok(())
}

#[test]
fn test_toml_unfixable_empty_array() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
unfixable = []
"#,
        ),
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
    ])?;

    // Run with --fix flag - all fixable rules should be fixed
    let _ = &mut case
        .command()
        .arg("check")
        .arg(".")
        .arg("--fix")
        .arg("--allow-no-vcs")
        .run()
        .normalize_os_executable_name();

    let fixed_contents = case.read_file("test.R")?;
    insta::assert_snapshot!(fixed_contents,
        @"
    anyNA(x)
    anyDuplicated(x) > 0
    "
    );

    Ok(())
}

#[test]
fn test_invalid_toml_fixable_rule() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
fixable = ["invalid_rule_name"]
"#,
        ),
        ("test.R", "any(is.na(x))"),
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
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Unknown rules in field `fixable` in 'jarl.toml': invalid_rule_name
    "
    );

    Ok(())
}

#[test]
fn test_invalid_toml_unfixable_rule() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
unfixable = ["invalid_rule_name"]
"#,
        ),
        ("test.R", "any(is.na(x))"),
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
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Unknown rules in field `unfixable` in 'jarl.toml': invalid_rule_name
    "
    );

    Ok(())
}

#[test]
fn test_toml_fixable_without_fix_flag() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
fixable = ["any_is_na"]
"#,
        ),
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
    ])?;

    // TODO: I guess here the message should say that only 1 violation is
    // fixable.
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
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_toml_fixable_with_select() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na", "any_duplicated", "length_levels"]
fixable = ["any_is_na"]
"#,
        ),
        (
            "test.R",
            "any(is.na(x))\nany(duplicated(x))\nlength(levels(x))",
        ),
    ])?;

    let _ = &mut case
        .command()
        .arg("check")
        .arg(".")
        .arg("--fix")
        .arg("--allow-no-vcs")
        .run()
        .normalize_os_executable_name();

    let fixed_contents = case.read_file("test.R")?;
    insta::assert_snapshot!(fixed_contents,
        @"
    anyNA(x)
    any(duplicated(x))
    length(levels(x))
    "
    );

    Ok(())
}

#[test]
fn test_toml_extend_select() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
extend-select = ["TESTTHAT"]
"#,
        ),
        (
            "test.R",
            "
any(is.na(x))
expect_equal(foo(x), TRUE)
",
        ),
    ])?;

    // Should detect both default rules (any_is_na) and TESTTHAT rules (expect_true_false)
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
fn test_toml_extend_select_with_select() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na"]
extend-select = ["TESTTHAT"]
"#,
        ),
        (
            "test.R",
            "
any(is.na(x))
any(duplicated(x))
expect_equal(foo(x), TRUE)
",
        ),
    ])?;

    // Should detect any_is_na (from select) and expect_true_false (from extend-select)
    // but NOT any_duplicated (not in select or extend-select)
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

    warning: expect_true_false
     --> test.R:4:1
      |
    4 | expect_equal(foo(x), TRUE)
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
fn test_toml_extend_select_unknown_rule() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
extend-select = ["FOO"]
"#,
        ),
        ("test.R", "any(is.na(x))"),
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
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Unknown rules in field `extend-select` in 'jarl.toml': FOO
    "
    );

    Ok(())
}

#[test]
fn test_include_single_file() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
include = ["included.R"]
"#,
        ),
        ("included.R", "any(is.na(x))"),
        ("excluded.R", "any(is.na(y))"),
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
     --> included.R:1:1
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
fn test_include_directory() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
include = ["R/"]
"#,
        ),
        ("R/utils.R", "any(is.na(x))"),
        ("test.R", "any(is.na(y))"),
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
     --> R/utils.R:1:1
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
fn test_include_glob_pattern() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
include = ["R-*.R"]
"#,
        ),
        ("R-utils.R", "any(is.na(x))"),
        ("R-helpers.R", "any(is.na(y))"),
        ("test.R", "any(is.na(z))"),
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
     --> R-helpers.R:1:1
      |
    1 | any(is.na(y))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_is_na
     --> R-utils.R:1:1
      |
    1 | any(is.na(x))
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
fn test_include_empty_array() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
include = []
"#,
        ),
        ("test.R", "any(is.na(x))"),
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


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_include_and_exclude() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
include = ["R/"]
exclude = ["R/generated.R"]
"#,
        ),
        ("R/utils.R", "any(is.na(x))"),
        ("R/generated.R", "any(is.na(y))"),
        ("test.R", "any(is.na(z))"),
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
     --> R/utils.R:1:1
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
fn test_include_rmd_qmd_glob() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
include = ["**/*.{Rmd,qmd}"]
"#,
        ),
        (
            "report.Rmd",
            "---\ntitle: \"Test\"\n---\n\n```{r}\nany(is.na(x))\n```\n",
        ),
        (
            "analysis.qmd",
            "---\ntitle: \"Test\"\n---\n\n```{r}\nany(is.na(y))\n```\n",
        ),
        ("plain.R", "any(is.na(z))"),
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
     --> analysis.qmd:6:1
      |
    6 | any(is.na(y))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_is_na
     --> report.Rmd:6:1
      |
    6 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_include_wrong_values() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "jarl.toml",
        r#"
[lint]
include = true
"#,
    )?;

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
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Failed to parse [TEMP_DIR]/jarl.toml:
    TOML parse error at line 3, column 11
      |
    3 | include = true
      |           ^^^^
    invalid type: boolean `true`, expected a sequence
    "
    );

    case.write_file(
        "jarl.toml",
        r#"
[lint]
include = ["a", 1]
"#,
    )?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @r#"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Failed to parse [TEMP_DIR]/jarl.toml:
    TOML parse error at line 3, column 17
      |
    3 | include = ["a", 1]
      |                 ^
    invalid type: integer `1`, expected a string
    "#
    );

    Ok(())
}

// --- Hierarchical configuration tests ---

/// When a subdirectory has its own jarl.toml, `jarl check .` should use the
/// nearest config for each file: root files use the root config, subfolder
/// files use the subfolder config.
#[test]
fn test_hierarchical_toml_dir_uses_nearest_config() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na"]
"#,
        ),
        ("root.R", "any(is.na(x))\nany(duplicated(x))"),
        (
            "subfolder/jarl.toml",
            r#"
[lint]
select = ["any_duplicated"]
"#,
        ),
        ("subfolder/sub.R", "any(is.na(x))\nany(duplicated(x))"),
    ])?;

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
    warning: any_is_na
     --> root.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_duplicated
     --> subfolder/sub.R:2:1
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

/// When a subdirectory has no jarl.toml of its own, files there should fall
/// back to the nearest ancestor config (i.e. the root jarl.toml).
#[test]
fn test_hierarchical_toml_subdir_inherits_root_config() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na"]
"#,
        ),
        ("root.R", "any(is.na(x))\nany(duplicated(x))"),
        ("subfolder/sub.R", "any(is.na(x))\nany(duplicated(x))"),
    ])?;

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
    warning: any_is_na
     --> root.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_is_na
     --> subfolder/sub.R:1:1
      |
    1 | any(is.na(x))
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

/// Passing individual file paths (e.g. from shell glob expansion) should work
/// the same as `jarl check .`: each file uses the nearest jarl.toml above it.
#[test]
fn test_hierarchical_toml_individual_files_use_nearest_config() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na"]
"#,
        ),
        ("root.R", "any(is.na(x))\nany(duplicated(x))"),
        (
            "subfolder/jarl.toml",
            r#"
[lint]
select = ["any_duplicated"]
"#,
        ),
        ("subfolder/sub.R", "any(is.na(x))\nany(duplicated(x))"),
    ])?;

    // Pass both files explicitly, as a shell glob would expand them
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("root.R")
            .arg("subfolder/sub.R")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> root.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_duplicated
     --> subfolder/sub.R:2:1
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

/// When a subfolder has its own jarl.toml with `exclude` patterns, running
/// `jarl check .` from the parent should respect the subfolder's exclude list.
#[test]
fn test_hierarchical_toml_subfolder_exclude_respected() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na"]
"#,
        ),
        ("root.R", "any(is.na(x))"),
        (
            "subfolder/jarl.toml",
            r#"
[lint]
select = ["any_is_na"]
exclude = ["bar.R"]
"#,
        ),
        ("subfolder/foo.R", "any(is.na(x))"),
        ("subfolder/bar.R", "any(is.na(x))"),
    ])?;

    // bar.R should be excluded by the subfolder config;
    // only root.R and subfolder/foo.R should be flagged
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
    warning: any_is_na
     --> root.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_is_na
     --> subfolder/foo.R:1:1
      |
    1 | any(is.na(x))
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

/// When a subfolder has its own jarl.toml with `include` patterns, running
/// `jarl check .` from the parent should only lint matching files in that subfolder.
#[test]
fn test_hierarchical_toml_subfolder_include_respected() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na"]
"#,
        ),
        ("root.R", "any(is.na(x))"),
        (
            "subfolder/jarl.toml",
            r#"
[lint]
select = ["any_is_na"]
include = ["foo.R"]
"#,
        ),
        ("subfolder/foo.R", "any(is.na(x))"),
        ("subfolder/bar.R", "any(is.na(x))"),
    ])?;

    // bar.R should be excluded because only foo.R is included;
    // only root.R and subfolder/foo.R should be flagged
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
    warning: any_is_na
     --> root.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_is_na
     --> subfolder/foo.R:1:1
      |
    1 | any(is.na(x))
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
