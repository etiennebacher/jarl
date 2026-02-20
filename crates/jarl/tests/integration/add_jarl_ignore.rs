use std::process::Command;
use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_add_jarl_ignore_reason_with_newlines() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(directory.join(test_path), "any(is.na(x))\n")?;

    // Reason contains newlines - they should be converted to spaces
    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore=line1\nline2\nline3")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: false
exit_code: 255
----- stdout -----

----- stderr -----
jarl failed
  Cause: --add-jarl-ignore=<reason> cannot contain newline characters.
"
                            );

    // Check the file content - newlines should be converted to spaces
    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!(
        content,
        @r"any(is.na(x))
    "
    );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_basic() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(directory.join(test_path), "any(is.na(x))\n")?;

    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Modified: Added 1 suppression comment(s) to test.R

Summary: Added 1 suppression comment(s) across 1 file(s).

----- stderr -----
"
                            );

    // Check the file content after modification
    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!(
                            content,
                                @r"# jarl-ignore any_is_na: <reason>
any(is.na(x))
"
                            );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_custom_reason() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(directory.join(test_path), "any(is.na(x))\n")?;

    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore=known issue in legacy code")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Modified: Added 1 suppression comment(s) to test.R

Summary: Added 1 suppression comment(s) across 1 file(s).

----- stderr -----
"
                            );

    // Check the file content after modification
    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!(
                            content,
                                @r"# jarl-ignore any_is_na: known issue in legacy code
any(is.na(x))
"
                            );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_multiple_violations_one_file() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "any(is.na(x))
any(duplicated(y))
",
    )?;

    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Modified: Added 2 suppression comment(s) to test.R

Summary: Added 2 suppression comment(s) across 1 file(s).

----- stderr -----
"
                            );

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!(
                            content,
                                @r"# jarl-ignore any_is_na: <reason>
any(is.na(x))
# jarl-ignore any_duplicated: <reason>
any(duplicated(y))
"
                            );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_multiple_files() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(directory.join("file1.R"), "any(is.na(x))\n")?;
    std::fs::write(directory.join("file2.R"), "any(duplicated(y))\n")?;

    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Modified: Added 1 suppression comment(s) to file1.R
Modified: Added 1 suppression comment(s) to file2.R

Summary: Added 2 suppression comment(s) across 2 file(s).

----- stderr -----
"
                            );

    let content1 = std::fs::read_to_string(directory.join("file1.R"))?;
    let content2 = std::fs::read_to_string(directory.join("file2.R"))?;
    insta::assert_snapshot!(
                            content1,
                                @r"# jarl-ignore any_is_na: <reason>
any(is.na(x))
"
                            );
    insta::assert_snapshot!(
                            content2,
                                @r"# jarl-ignore any_duplicated: <reason>
any(duplicated(y))
"
                            );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_no_violations() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(directory.join(test_path), "x <- 1\n")?;

    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Info: No violations found, no suppression comments added.

----- stderr -----
"
                            );

    // File should be unchanged
    let content = std::fs::read_to_string(directory.join(test_path))?;
    assert_eq!(content, "x <- 1\n");

    Ok(())
}

#[test]
fn test_add_jarl_ignore_idempotent() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(directory.join(test_path), "any(is.na(x))\n")?;

    // First run
    Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run();

    let content_after_first = std::fs::read_to_string(directory.join(test_path))?;

    // Second run - should not add duplicate comments
    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Info: No violations found, no suppression comments added.

----- stderr -----
"
                            );

    let content_after_second = std::fs::read_to_string(directory.join(test_path))?;

    // Content should be unchanged after second run
    assert_eq!(content_after_first, content_after_second);
    insta::assert_snapshot!(
                            content_after_second,
                                @r"# jarl-ignore any_is_na: <reason>
any(is.na(x))
"
                            );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_nested_violation() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "x <- foo(any(is.na(y)))
",
    )?;

    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Modified: Added 1 suppression comment(s) to test.R

Summary: Added 1 suppression comment(s) across 1 file(s).

----- stderr -----
"
                            );

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!(
                            content,
                                @r"# jarl-ignore any_is_na: <reason>
x <- foo(any(is.na(y)))
"
                            );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_with_indentation() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "f <- function() {
  any(is.na(x))
}
",
    )?;

    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Modified: Added 1 suppression comment(s) to test.R

Summary: Added 1 suppression comment(s) across 1 file(s).

----- stderr -----
"
                            );

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!(
                            content,
                                @r"f <- function() {
  # jarl-ignore any_is_na: <reason>
  any(is.na(x))
}
"
                            );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_function_parameter() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "f <- function(
    a = any(is.na(x))
) {
  1
}
",
    )?;

    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Modified: Added 1 suppression comment(s) to test.R

Summary: Added 1 suppression comment(s) across 1 file(s).

----- stderr -----
"
                            );

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!(
                            content,
                                @r"f <- function(
    # jarl-ignore any_is_na: <reason>
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
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "if (any(is.na(x))) {
  print(1)
}
",
    )?;

    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Modified: Added 1 suppression comment(s) to test.R

Summary: Added 1 suppression comment(s) across 1 file(s).

----- stderr -----
"
                            );

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!(
                            content,
                                @r"if (
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
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        r#"x |>
  foo() |>
  download.file(mode = "w") |>
  bar()
"#,
    )?;

    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Modified: Added 1 suppression comment(s) to test.R

Summary: Added 1 suppression comment(s) across 1 file(s).

----- stderr -----
"
                            );

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!(
        content,
        @r#"x |>
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
    // Two violations of the same rule in an if condition should produce one comment
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(directory.join(test_path), "z <- x == TRUE && any(is.na(y))")?;

    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Modified: Added 2 suppression comment(s) to test.R

Summary: Added 2 suppression comment(s) across 1 file(s).

----- stderr -----
"
                            );

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!(
                            content,
                                @r"# jarl-ignore redundant_equals: <reason>
# jarl-ignore any_is_na: <reason>
z <- x == TRUE && any(is.na(y))
"
                            );

    insta::assert_snapshot!(
                                Command::new(binary_path())
                                                        .current_dir(directory)
                                                        .arg("check")
                                                        .arg(".")
                                                        .run()
                                                        .normalize_os_executable_name()
                                                        .normalize_temp_paths(),
                        @r"
success: true
exit_code: 0
----- stdout -----
All checks passed!

----- stderr -----
"
                            );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_same_rule_same_line_in_if_condition() -> anyhow::Result<()> {
    // Two violations of the same rule in an if condition should produce one comment
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "if (x == TRUE || y == TRUE) {
  1
}
",
    )?;

    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Modified: Added 1 suppression comment(s) to test.R

Summary: Added 1 suppression comment(s) across 1 file(s).

----- stderr -----
"
                            );

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!(
                            content,
                                @r"if (
    # jarl-ignore redundant_equals: <reason>
    x == TRUE || y == TRUE) {
  1
}
"
                            );

    insta::assert_snapshot!(
                                Command::new(binary_path())
                                                        .current_dir(directory)
                                                        .arg("check")
                                                        .arg(".")
                                                        .run()
                                                        .normalize_os_executable_name()
                                                        .normalize_temp_paths(),
                        @r"
success: true
exit_code: 0
----- stdout -----
All checks passed!

----- stderr -----
"
                            );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_different_rules_same_line() -> anyhow::Result<()> {
    // Two violations of different rules in an if condition should produce one comment with both rules
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "if (x == TRUE || any(is.na(y))) {
  1
}
",
    )?;

    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Modified: Added 2 suppression comment(s) to test.R

Summary: Added 2 suppression comment(s) across 1 file(s).

----- stderr -----
"
                            );

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!(
                            content,
                                @r"if (
    # jarl-ignore redundant_equals: <reason>
    # jarl-ignore any_is_na: <reason>
    x == TRUE || any(is.na(y))) {
  1
}
"
                            );

    insta::assert_snapshot!(
                                Command::new(binary_path())
                                                        .current_dir(directory)
                                                        .arg("check")
                                                        .arg(".")
                                                        .run()
                                                        .normalize_os_executable_name()
                                                        .normalize_temp_paths(),
                        @r"
success: true
exit_code: 0
----- stdout -----
All checks passed!

----- stderr -----
"
                            );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_multiline_condition() -> anyhow::Result<()> {
    // Multi-line condition with violations on different lines should produce one comment
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "if (
  super_long_variable_name == TRUE ||
    super_long_variable_name_again == TRUE
) {
  1
}
",
    )?;

    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Modified: Added 1 suppression comment(s) to test.R

Summary: Added 1 suppression comment(s) across 1 file(s).

----- stderr -----
"
                            );

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!(
                            content,
                                @r"if (
  # jarl-ignore redundant_equals: <reason>
  super_long_variable_name == TRUE ||
    super_long_variable_name_again == TRUE
) {
  1
}
"
                            );

    insta::assert_snapshot!(
                                Command::new(binary_path())
                                    .current_dir(directory)
                                    .arg("check")
                                    .arg(".")
                                    .run()
                                    .normalize_os_executable_name()
                                    .normalize_temp_paths(),
                        @r"
success: true
exit_code: 0
----- stdout -----
All checks passed!

----- stderr -----
"
                            );

    Ok(())
}

#[test]
fn test_add_jarl_ignore_rmd_basic_insertion() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.Rmd";
    std::fs::write(
        directory.join(test_path),
        "---\ntitle: Test\n---\n\n```{r}\nany(is.na(x))\n```\n",
    )?;

    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Modified: Added 1 suppression comment(s) to test.Rmd

Summary: Added 1 suppression comment(s) across 1 file(s).

----- stderr -----
"
                            );

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!(
                            content,
                                @r"---
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
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.Rmd";
    std::fs::write(
        directory.join(test_path),
        "```{r}\nany(is.na(x))\n```\n\n```{r}\n1 + 1\nany(is.na(y))\n```\n",
    )?;

    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Modified: Added 2 suppression comment(s) to test.Rmd

Summary: Added 2 suppression comment(s) across 1 file(s).

----- stderr -----
"
                            );

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!(
                            content,
                                @r"```{r}
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
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.qmd";
    std::fs::write(
        directory.join(test_path),
        "---\ntitle: Test\n---\n\n```{r}\nany(is.na(x))\n```\n",
    )?;

    let output = Command::new(binary_path())
        .current_dir(directory)
        .arg("check")
        .arg(".")
        .arg("--add-jarl-ignore")
        .run()
        .normalize_os_executable_name()
        .normalize_temp_paths();

    insta::assert_snapshot!(
                                output,
                                @r"
success: true
exit_code: 0
----- stdout -----
Modified: Added 1 suppression comment(s) to test.qmd

Summary: Added 1 suppression comment(s) across 1 file(s).

----- stderr -----
"
                            );

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!(
                            content,
                                @r"---
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
