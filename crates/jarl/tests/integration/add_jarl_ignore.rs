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

    insta::assert_snapshot!("reason_with_newlines_output", output);

    // Check the file content - newlines should be converted to spaces
    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!("reason_with_newlines_file_content", content);

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

    insta::assert_snapshot!("basic_output", output);

    // Check the file content after modification
    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!("basic_file_content", content);

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

    insta::assert_snapshot!("custom_reason_output", output);

    // Check the file content after modification
    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!("custom_reason_file_content", content);

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

    insta::assert_snapshot!("multiple_violations_output", output);

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!("multiple_violations_file_content", content);

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

    insta::assert_snapshot!("multiple_files_output", output);

    let content1 = std::fs::read_to_string(directory.join("file1.R"))?;
    let content2 = std::fs::read_to_string(directory.join("file2.R"))?;
    insta::assert_snapshot!("multiple_files_file1_content", content1);
    insta::assert_snapshot!("multiple_files_file2_content", content2);

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

    insta::assert_snapshot!("no_violations_output", output);

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

    insta::assert_snapshot!("idempotent_output", output);

    let content_after_second = std::fs::read_to_string(directory.join(test_path))?;

    // Content should be unchanged after second run
    assert_eq!(content_after_first, content_after_second);
    insta::assert_snapshot!("idempotent_file_content", content_after_second);

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

    insta::assert_snapshot!("nested_violation_output", output);

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!("nested_violation_file_content", content);

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

    insta::assert_snapshot!("indentation_output", output);

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!("indentation_file_content", content);

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

    insta::assert_snapshot!("function_parameter_output", output);

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!("function_parameter_file_content", content);

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

    insta::assert_snapshot!("inline_condition_output", output);

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!("inline_condition_file_content", content);

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

    insta::assert_snapshot!("pipe_chain_output", output);

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!("pipe_chain_file_content", content);

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

    insta::assert_snapshot!("same_rule_same_line_output", output);

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!("same_rule_same_line_file_content", content);

    insta::assert_snapshot!(
        "same_rule_same_line_jarl_check",
        Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
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

    insta::assert_snapshot!("same_rule_same_line_in_if_output", output);

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!("same_rule_same_line_in_if_file_content", content);

    insta::assert_snapshot!(
        "same_rule_same_line_in_if_jarl_check",
        Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
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

    insta::assert_snapshot!("different_rules_same_line_output", output);

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!("different_rules_same_line_file_content", content);

    insta::assert_snapshot!(
        "different_rules_same_line_jarl_check",
        Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
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

    insta::assert_snapshot!("multiline_condition_output", output);

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!("multiline_condition_file_content", content);

    insta::assert_snapshot!(
        "multiline_condition_jarl_check",
        Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
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

    insta::assert_snapshot!("rmd_basic_insertion_output", output);

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!("rmd_basic_insertion_file_content", content);

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

    insta::assert_snapshot!("rmd_multiple_chunks_output", output);

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!("rmd_multiple_chunks_file_content", content);

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

    insta::assert_snapshot!("qmd_insertion_output", output);

    let content = std::fs::read_to_string(directory.join(test_path))?;
    insta::assert_snapshot!("qmd_insertion_file_content", content);

    Ok(())
}
