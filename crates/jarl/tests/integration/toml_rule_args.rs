use crate::helpers::{CliTest, CommandExt};

// assignment ----------------------------------------

#[test]
fn test_assignment_unknown_field_is_error() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint.assignment]
unknown-option = "foo"
"#,
        ),
        ("test.R", "x <- 1"),
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
      Cause: Failed to parse [TEMP_DIR]/jarl.toml:
    TOML parse error at line 3, column 1
      |
    3 | unknown-option = "foo"
      | ^^^^^^^^^^^^^^
    unknown field `unknown-option`, expected `operator`
    "#
    );

    Ok(())
}

// duplicated_arguments ----------------------------------------

#[test]
fn test_duplicated_arguments_both_skipped_and_extend_is_error() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]

[lint.duplicated_arguments]
skipped-functions = ["list"]
extend-skipped-functions = ["my_fun"]
"#,
        ),
        ("test.R", "list(a = 1, a = 2)"),
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
    Cannot specify both `skipped-functions` and `extend-skipped-functions` in `[lint.duplicated_arguments]`.
    "
    );

    Ok(())
}

#[test]
fn test_duplicated_arguments_unknown_field_is_error() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]

[lint.duplicated_arguments]
unknown-option = ["list"]
"#,
        ),
        ("test.R", "list(a = 1, a = 2)"),
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
      Cause: Failed to parse [TEMP_DIR]/jarl.toml:
    TOML parse error at line 5, column 1
      |
    5 | unknown-option = ["list"]
      | ^^^^^^^^^^^^^^
    unknown field `unknown-option`, expected `skipped-functions` or `extend-skipped-functions`
    "#
    );

    Ok(())
}

// implicit_assignment ----------------------------------------

#[test]
fn test_implicit_assignment_both_skipped_and_extend_is_error() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]

[lint.implicit_assignment]
skipped-functions = ["list"]
extend-skipped-functions = ["my_fun"]
"#,
        ),
        ("test.R", "list(a = 1, a = 2)"),
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
    Cannot specify both `skipped-functions` and `extend-skipped-functions` in `[lint.implicit_assignment]`.
    "
    );

    Ok(())
}

#[test]
fn test_implicit_assignment_unknown_field_is_error() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]

[lint.implicit_assignment]
unknown-option = ["list"]
"#,
        ),
        ("test.R", "list(a = 1, a = 2)"),
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
      Cause: Failed to parse [TEMP_DIR]/jarl.toml:
    TOML parse error at line 5, column 1
      |
    5 | unknown-option = ["list"]
      | ^^^^^^^^^^^^^^
    unknown field `unknown-option`, expected `skipped-functions` or `extend-skipped-functions`
    "#
    );

    Ok(())
}

// missing_argument ----------------------------------------

#[test]
fn test_missing_argument_both_skipped_and_extend_is_error() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]

[lint.missing_argument]
skipped-functions = ["list"]
extend-skipped-functions = ["my_fun"]
"#,
        ),
        ("test.R", "list(a = 1, a = 2)"),
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
    Cannot specify both `skipped-functions` and `extend-skipped-functions` in `[lint.missing_argument]`.
    "
    );

    Ok(())
}

// nested_pipe ----------------------------------------

#[test]
fn test_nested_pipe_both_skipped_and_extend_is_error() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]

[lint.nested_pipe]
skipped-functions = ["try"]
extend-skipped-functions = ["my_fun"]
"#,
        ),
        ("test.R", "print(a %>% b())"),
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
    Cannot specify both `skipped-functions` and `extend-skipped-functions` in `[lint.nested_pipe]`.
    "
    );

    Ok(())
}

#[test]
fn test_nested_pipe_unknown_field_is_error() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]

[lint.nested_pipe]
unknown-option = ["try"]
"#,
        ),
        ("test.R", "print(a %>% b())"),
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
      Cause: Failed to parse [TEMP_DIR]/jarl.toml:
    TOML parse error at line 5, column 1
      |
    5 | unknown-option = ["try"]
      | ^^^^^^^^^^^^^^
    unknown field `unknown-option`, expected `skipped-functions` or `extend-skipped-functions`
    "#
    );

    Ok(())
}

// pipe_consistency ----------------------------------------

#[test]
fn test_pipe_consistency_unknown_field_is_error() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]

[lint.pipe_consistency]
unknown-option = "x"
"#,
        ),
        ("test.R", "1 + 1"),
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
      Cause: Failed to parse [TEMP_DIR]/jarl.toml:
    TOML parse error at line 5, column 1
      |
    5 | unknown-option = "x"
      | ^^^^^^^^^^^^^^
    unknown field `unknown-option`, expected `pipe`
    "#
    );

    Ok(())
}

#[test]
fn test_pipe_consistency_invalid_quote_is_error() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
extend-select = ["pipe_consistency"]

[lint.pipe_consistency]
pipe = "foo"
"#,
        ),
        ("test.R", "1 + 1"),
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
      Cause: Invalid configuration in [TEMP_DIR]/jarl.toml:
    Invalid value for `pipe` in `[lint.pipe_consistency]`: "foo". Expected "|>" or "%>%".
    "#
    );

    Ok(())
}

// quotes ----------------------------------------

#[test]
fn test_quotes_unknown_field_is_error() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]

[lint.quotes]
unknown-option = "x"
"#,
        ),
        ("test.R", "'x'"),
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
      Cause: Failed to parse [TEMP_DIR]/jarl.toml:
    TOML parse error at line 5, column 1
      |
    5 | unknown-option = "x"
      | ^^^^^^^^^^^^^^
    unknown field `unknown-option`, expected `quote`
    "#
    );

    Ok(())
}

#[test]
fn test_quotes_invalid_quote_is_error() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]
extend-select = ["quotes"]

[lint.quotes]
quote = "foo"
"#,
        ),
        ("test.R", "'x'"),
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
      Cause: Invalid configuration in [TEMP_DIR]/jarl.toml:
    Invalid value for `quote` in `[lint.quotes]`: "foo". Expected "double" or "single".
    "#
    );

    Ok(())
}

// unreachable_code ----------------------------------------

#[test]
fn test_unreachable_code_both_stopping_and_extend_is_error() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "jarl.toml",
            r#"
[lint]

[lint.unreachable_code]
stopping-functions = ["stop"]
extend-stopping-functions = ["my_stop"]
"#,
        ),
        (
            "test.R",
            r#"
foo <- function() {
  stop("error")
  1 + 1
}
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
    Cannot specify both `stopping-functions` and `extend-stopping-functions` in `[lint.unreachable_code]`.
    "
    );

    Ok(())
}
