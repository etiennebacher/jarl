use crate::helpers::{CliTest, CommandExt};

/// A plain pattern ignores the listed rules only in files it matches. The same
/// violation in a non-matching file is still reported.
#[test]
fn test_plain_pattern_ignores_in_matching_file() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("foo.R", "any(is.na(x))\n"),
        ("bar.R", "any(is.na(x))\n"),
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na"]

[lint.per-file-ignores]
"foo.R" = ["any_is_na"]
"#,
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
     --> bar.R:1:1
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
fn test_plain_pattern_ignores_in_matching_folder() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("R/foo.R", "any(is.na(x))\n"),
        ("bar.R", "any(is.na(x))\n"),
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na"]

[lint.per-file-ignores]
"R/" = ["any_is_na"]
"#,
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
     --> bar.R:1:1
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

/// A negated pattern (`!`) ignores the listed rules in files that do NOT match
/// the pattern, i.e. everywhere but the matched location.
#[test]
fn test_negated_pattern_ignores_outside_match() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("R/foo.R", "any(is.na(x))\n"),
        ("src/bar.R", "any(is.na(x))\n"),
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na"]

[lint.per-file-ignores]
# ignore everywhere but in the R folder
"!R/**" = ["any_is_na"]
"#,
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
     --> R/foo.R:1:1
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

/// When several patterns match a file, the rules from all of them are ignored.
#[test]
fn test_file_matches_several_patterns() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("R/foo.R", "any(is.na(x))\nany(duplicated(x))\n"),
        ("src/bar.R", "any(is.na(x))\nany(duplicated(x))\n"),
        (
            "jarl.toml",
            r#"
[lint]
select = ["any_is_na", "any_duplicated"]

[lint.per-file-ignores]
"R/" = ["any_is_na"]
"*/*.R" = ["any_duplicated"]
"#,
        ),
    ])?;

    // "R/foo.R" matches the two patterns so no diagnostics are reported there.
    // "src/bar.R" only matches the second pattern
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
     --> src/bar.R:1:1
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

/// `[lint.per-file-ignores]` is a TOML sub-table, so any bare `[lint]` key
/// written *after* it is absorbed into the table as a glob pattern. When that
/// key's value is not an array of rule names (e.g. `default-exclude = false`),
/// TOML deserialization fails with a type error rather than silently dropping
/// the option. The fix is to move scalar `[lint]` keys above the table.
#[test]
fn test_scalar_option_after_table_errors() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("foo.R", "any(is.na(x))\n"),
        (
            "jarl.toml",
            r#"
[lint.per-file-ignores]
"tests/" = ["PERF"]

default-exclude = false
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
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Failed to parse [TEMP_DIR]/jarl.toml:
    TOML parse error at line 5, column 19
      |
    5 | default-exclude = false
      |                   ^^^^^
    invalid type: boolean `false`, expected a sequence
    "
    );

    Ok(())
}

/// An unknown rule name in `per-file-ignores` is a configuration error.
#[test]
fn test_unknown_rule_name_errors() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("foo.R", "any(is.na(x))\n"),
        (
            "jarl.toml",
            r#"
[lint.per-file-ignores]
"foo.R" = ["not_a_real_rule"]
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
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Invalid configuration in [TEMP_DIR]/jarl.toml:
    Unknown rules in `per-file-ignores` for pattern 'foo.R': not_a_real_rule
    "
    );

    Ok(())
}
