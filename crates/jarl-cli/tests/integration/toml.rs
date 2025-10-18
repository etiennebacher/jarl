use std::process::Command;

use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_empty_toml_uses_all_rules() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))\nany(duplicated(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    // Empty TOML with just [linter] section
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[linter]
"#,
    )?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
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
fn test_empty_select_array() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // TOML with explicitly empty select array (should select no rules)
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[linter]
select = []
"#,
    )?;

    let test_path = "test.R";
    let test_contents = "any(is.na(x))\nany(duplicated(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
    );

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[linter]
select = [""]
"#,
    )?;

    let test_path = "test.R";
    let test_contents = "any(is.na(x))\nany(duplicated(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
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
fn test_empty_ignore_array() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // TOML with explicitly empty ignore array (should ignore no rules)
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[linter]
ignore = []
"#,
    )?;

    let test_path = "test.R";
    let test_contents = "any(is.na(x))\nany(duplicated(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
    );

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[linter]
ignore = [""]
"#,
    )?;

    let test_path = "test.R";
    let test_contents = "any(is.na(x))\nany(duplicated(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_toml_select_rules() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // TOML that only selects any_is_na rule
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[linter]
select = ["any_is_na"]
"#,
    )?;

    let test_path = "test.R";
    let test_contents = "any(is.na(x))\nany(duplicated(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_toml_ignore_rules() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // TOML that ignores any_duplicated rule
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[linter]
ignore = ["any_duplicated"]
"#,
    )?;

    let test_path = "test.R";
    let test_contents = "any(is.na(x))\nany(duplicated(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
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
fn test_toml_select_and_ignore() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // TOML with both select and ignore
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[linter]
select = ["any_is_na", "any_duplicated", "length_levels"]
ignore = ["length_levels"]
"#,
    )?;

    let test_path = "test.R";
    let test_contents = r#"any(is.na(x))
any(duplicated(x))
length(levels(x))"#;
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
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
fn test_cli_select_overrides_toml() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // TOML selects any_is_na
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[linter]
select = ["any_is_na"]
ignore = ["length_levels"]
"#,
    )?;

    let test_path = "test.R";
    let test_contents = r#"any(is.na(x))
any(duplicated(x))
length(levels(x))"#;
    std::fs::write(directory.join(test_path), test_contents)?;

    // CLI select should override TOML select, but TOML ignore should still apply
    // TODO: not sure this is correct, length_levels is ignored but since it's
    // put explicitly in the CLI maybe it should raise?
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select-rules")
            .arg("any_duplicated,length_levels")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
    );

    Ok(())
}

#[test]
fn test_cli_ignore_adds_to_toml() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // TOML selects specific rules and ignores one
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[linter]
select = ["any_is_na", "any_duplicated", "length_levels"]
ignore = ["length_levels"]
"#,
    )?;

    let test_path = "test.R";
    let test_contents = r#"any(is.na(x))
any(duplicated(x))
length(levels(x))"#;
    std::fs::write(directory.join(test_path), test_contents)?;

    // CLI ignore should add to TOML ignore, using TOML select
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--ignore-rules")
            .arg("any_is_na")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
    );

    Ok(())
}

#[test]
fn test_cli_overrides_toml_completely() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // TOML with specific configuration
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[linter]
select = ["any_is_na"]
ignore = ["any_duplicated"]
"#,
    )?;

    let test_path = "test.R";
    let test_contents = r#"any(is.na(x))
any(duplicated(x))
length(levels(x))"#;
    std::fs::write(directory.join(test_path), test_contents)?;

    // Both CLI select and ignore should completely override TOML
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select-rules")
            .arg("length_levels,any_duplicated")
            .arg("--ignore-rules")
            .arg("length_levels")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
    );

    Ok(())
}

#[test]
fn test_invalid_toml_select_rule() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // TOML with invalid rule name
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[linter]
select = ["any_is_na", "foo"]
"#,
    )?;

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
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
fn test_invalid_toml_ignore_rule() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // TOML with invalid ignore rule
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[linter]
ignore = ["foo", "bar"]
"#,
    )?;

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
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
fn test_malformed_toml_syntax() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // Malformed TOML syntax
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[linter
select = ["any_is_na"
"#,
    )?;

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
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
fn test_unknown_toml_field() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // TOML with unknown field (should be rejected due to deny_unknown_fields)
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[linter]
select = ["any_is_na"]
unknown_field = ["value"]
"#,
    )?;

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
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
fn test_toml_without_linter_section() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // TOML without linter section (should use all rules)
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
# Just a comment, no linter section
"#,
    )?;

    let test_path = "test.R";
    let test_contents = "any(is.na(x))\nany(duplicated(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_empty_string_in_toml_ignore() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // TOML with empty string in ignore array (should error)
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[linter]
ignore = ["any_duplicated", "", "any_is_na"]
"#,
    )?;

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
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
fn test_whitespace_only_in_toml_select() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // TOML with whitespace-only string in select array (should error)
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[linter]
select = ["any_is_na", "   ", "any_duplicated"]
"#,
    )?;

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
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
fn test_no_toml_file_uses_all_rules() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // No TOML file at all (should use all rules)
    let test_path = "test.R";
    let test_contents = "any(is.na(x))\nany(duplicated(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}
