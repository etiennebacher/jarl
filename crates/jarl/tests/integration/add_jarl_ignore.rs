use crate::helpers::{CliTest, CommandExt};

#[test]
fn test_add_jarl_ignore_reason_with_newlines() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "any(is.na(x))\n")?;

    // Reason contains newlines - they should be converted to spaces
    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore=line1\nline2\nline3")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: --add-jarl-ignore=<reason> cannot contain newline characters.
    "
    );

    // Check the file content - newlines should be converted to spaces
    let content = case.read_file("test.R")?;
    insta::assert_snapshot!(
        content,
        @"any(is.na(x))"
    );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_basic() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "any(is.na(x))\n")?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Modified: Added 1 suppression comment(s) to test.R

    Summary: Added 1 suppression comment(s) across 1 file(s).

    ----- stderr -----
    "
    );

    // Check the file content after modification
    let content = case.read_file("test.R")?;
    insta::assert_snapshot!(
    content,
        @"
    # jarl-ignore any_is_na: <reason>
    any(is.na(x))
    "
    );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_custom_reason() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "any(is.na(x))\n")?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore=known issue in legacy code")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Modified: Added 1 suppression comment(s) to test.R

    Summary: Added 1 suppression comment(s) across 1 file(s).

    ----- stderr -----
    "
    );

    // Check the file content after modification
    let content = case.read_file("test.R")?;
    insta::assert_snapshot!(
    content,
        @"
    # jarl-ignore any_is_na: known issue in legacy code
    any(is.na(x))
    "
    );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_multiple_violations_one_file() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.R",
        "any(is.na(x))
any(duplicated(y))
",
    )?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Modified: Added 2 suppression comment(s) to test.R

    Summary: Added 2 suppression comment(s) across 1 file(s).

    ----- stderr -----
    "
    );

    let content = case.read_file("test.R")?;
    insta::assert_snapshot!(
    content,
        @"
    # jarl-ignore any_is_na: <reason>
    any(is.na(x))
    # jarl-ignore any_duplicated: <reason>
    any(duplicated(y))
    "
    );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_multiple_files() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("file1.R", "any(is.na(x))\n"),
        ("file2.R", "any(duplicated(y))\n"),
    ])?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Modified: Added 1 suppression comment(s) to file1.R
    Modified: Added 1 suppression comment(s) to file2.R

    Summary: Added 2 suppression comment(s) across 2 file(s).

    ----- stderr -----
    "
    );

    let content1 = case.read_file("file1.R")?;
    let content2 = case.read_file("file2.R")?;
    insta::assert_snapshot!(
    content1,
        @"
    # jarl-ignore any_is_na: <reason>
    any(is.na(x))
    "
    );
    insta::assert_snapshot!(
    content2,
        @"
    # jarl-ignore any_duplicated: <reason>
    any(duplicated(y))
    "
    );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_no_violations() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "1 + 1\n")?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Info: No violations found, no suppression comments added.

    ----- stderr -----
    "
    );

    // File should be unchanged
    let content = case.read_file("test.R")?;
    assert_eq!(content, "1 + 1\n");

    Ok(())
}

#[test]
fn test_add_jarl_ignore_idempotent() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "any(is.na(x))\n")?;

    // First run
    case.command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run();

    let content_after_first = case.read_file("test.R")?;

    // Second run - should not add duplicate comments
    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Info: No violations found, no suppression comments added.

    ----- stderr -----
    "
    );

    let content_after_second = case.read_file("test.R")?;

    // Content should be unchanged after second run
    assert_eq!(content_after_first, content_after_second);
    insta::assert_snapshot!(
    content_after_second,
        @"
    # jarl-ignore any_is_na: <reason>
    any(is.na(x))
    "
    );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_nested_violation() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "foo(any(is.na(y)))")?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Modified: Added 1 suppression comment(s) to test.R

    Summary: Added 1 suppression comment(s) across 1 file(s).

    ----- stderr -----
    "
    );

    let content = case.read_file("test.R")?;
    insta::assert_snapshot!(
    content,
        @"
    # jarl-ignore any_is_na: <reason>
    foo(any(is.na(y)))
    "
    );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_with_indentation() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.R",
        "f <- function() {
  any(is.na(x))
}
",
    )?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Modified: Added 1 suppression comment(s) to test.R

    Summary: Added 1 suppression comment(s) across 1 file(s).

    ----- stderr -----
    "
    );

    let content = case.read_file("test.R")?;
    insta::assert_snapshot!(
    content,
        @"
    f <- function() {
      # jarl-ignore any_is_na: <reason>
      any(is.na(x))
    }
    "
    );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_function_parameter() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.R",
        "f <- function(
    a = any(is.na(x))
) {
  1
}
",
    )?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Modified: Added 2 suppression comment(s) to test.R

    Summary: Added 2 suppression comment(s) across 1 file(s).

    ----- stderr -----
    "
    );

    let content = case.read_file("test.R")?;
    insta::assert_snapshot!(
    content,
        @"
    f <- function(
        # jarl-ignore any_is_na: <reason>
        # jarl-ignore unused_argument: <reason>
        a = any(is.na(x))
    ) {
      1
    }
    "
    );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_inline_condition() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.R",
        "if (any(is.na(x))) {
  print(1)
}
",
    )?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Modified: Added 1 suppression comment(s) to test.R

    Summary: Added 1 suppression comment(s) across 1 file(s).

    ----- stderr -----
    "
    );

    let content = case.read_file("test.R")?;
    insta::assert_snapshot!(
    content,
        @"
    if (
        # jarl-ignore any_is_na: <reason>
        any(is.na(x))) {
      print(1)
    }
    "
    );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_pipe_chain() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.R",
        r#"x |>
  foo() |>
  download.file(mode = "w") |>
  bar()
"#,
    )?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Modified: Added 1 suppression comment(s) to test.R

    Summary: Added 1 suppression comment(s) across 1 file(s).

    ----- stderr -----
    "
    );

    let content = case.read_file("test.R")?;
    insta::assert_snapshot!(
        content,
        @r#"
    x |>
      foo() |>
      # jarl-ignore download_file: <reason>
      download.file(mode = "w") |>
      bar()
    "#
    );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_same_rule_same_line() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "x == TRUE && any(is.na(y))")?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Modified: Added 2 suppression comment(s) to test.R

    Summary: Added 2 suppression comment(s) across 1 file(s).

    ----- stderr -----
    "
    );

    let content = case.read_file("test.R")?;
    insta::assert_snapshot!(
    content,
        @"
    # jarl-ignore redundant_equals: <reason>
    # jarl-ignore any_is_na: <reason>
    x == TRUE && any(is.na(y))
    "
    );

    insta::assert_snapshot!(
            case.command()
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

    Ok(())
}

#[test]
fn test_add_jarl_ignore_same_rule_same_line_in_if_condition() -> anyhow::Result<()> {
    // Two violations of the same rule in an if condition should produce one comment
    let case = CliTest::with_file(
        "test.R",
        "if (x == TRUE || y == TRUE) {
  1
}
",
    )?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Modified: Added 1 suppression comment(s) to test.R

    Summary: Added 1 suppression comment(s) across 1 file(s).

    ----- stderr -----
    "
    );

    let content = case.read_file("test.R")?;
    insta::assert_snapshot!(
    content,
        @"
    if (
        # jarl-ignore redundant_equals: <reason>
        x == TRUE || y == TRUE) {
      1
    }
    "
    );

    insta::assert_snapshot!(
            case.command()
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

    Ok(())
}

#[test]
fn test_add_jarl_ignore_different_rules_same_line() -> anyhow::Result<()> {
    // Two violations of different rules in an if condition should produce one comment with both rules
    let case = CliTest::with_file(
        "test.R",
        "if (x == TRUE || any(is.na(y))) {
  1
}
",
    )?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Modified: Added 2 suppression comment(s) to test.R

    Summary: Added 2 suppression comment(s) across 1 file(s).

    ----- stderr -----
    "
    );

    let content = case.read_file("test.R")?;
    insta::assert_snapshot!(
    content,
        @"
    if (
        # jarl-ignore redundant_equals: <reason>
        # jarl-ignore any_is_na: <reason>
        x == TRUE || any(is.na(y))) {
      1
    }
    "
    );

    insta::assert_snapshot!(
            case.command()
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

    Ok(())
}

#[test]
fn test_add_jarl_ignore_multiline_condition() -> anyhow::Result<()> {
    // Multi-line condition with violations on different lines should produce one comment
    let case = CliTest::with_file(
        "test.R",
        "if (
  super_long_variable_name == TRUE ||
    super_long_variable_name_again == TRUE
) {
  1
}
",
    )?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Modified: Added 1 suppression comment(s) to test.R

    Summary: Added 1 suppression comment(s) across 1 file(s).

    ----- stderr -----
    "
    );

    let content = case.read_file("test.R")?;
    insta::assert_snapshot!(
    content,
        @"
    if (
      # jarl-ignore redundant_equals: <reason>
      super_long_variable_name == TRUE ||
        super_long_variable_name_again == TRUE
    ) {
      1
    }
    "
    );

    insta::assert_snapshot!(
            case.command()
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

    Ok(())
}

#[test]
fn test_add_jarl_ignore_rmd_basic_insertion() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "---\ntitle: Test\n---\n\n```{r}\nany(is.na(x))\n```\n",
    )?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Modified: Added 1 suppression comment(s) to test.Rmd

    Summary: Added 1 suppression comment(s) across 1 file(s).

    ----- stderr -----
    "
    );

    let content = case.read_file("test.Rmd")?;
    insta::assert_snapshot!(
    content,
        @"
    ---
    title: Test
    ---

    ```{r}
    # jarl-ignore any_is_na: <reason>
    any(is.na(x))
    ```
    "
    );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_rmd_multiple_chunks() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "```{r}\nany(is.na(x))\n```\n\n```{r}\n1 + 1\nany(is.na(y))\n```\n",
    )?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Modified: Added 2 suppression comment(s) to test.Rmd

    Summary: Added 2 suppression comment(s) across 1 file(s).

    ----- stderr -----
    "
    );

    let content = case.read_file("test.Rmd")?;
    insta::assert_snapshot!(
    content,
        @"
    ```{r}
    # jarl-ignore any_is_na: <reason>
    any(is.na(x))
    ```

    ```{r}
    1 + 1
    # jarl-ignore any_is_na: <reason>
    any(is.na(y))
    ```
    "
    );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_qmd_insertion() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.qmd",
        "---\ntitle: Test\n---\n\n```{r}\nany(is.na(x))\n```\n",
    )?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
        output,
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Modified: Added 1 suppression comment(s) to test.qmd

    Summary: Added 1 suppression comment(s) across 1 file(s).

    ----- stderr -----
    "
    );

    let content = case.read_file("test.qmd")?;
    insta::assert_snapshot!(
    content,
        @"
    ---
    title: Test
    ---

    ```{r}
    # jarl-ignore any_is_na: <reason>
    any(is.na(x))
    ```
    "
    );

    Ok(())
}
