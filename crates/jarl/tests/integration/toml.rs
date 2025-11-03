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

    // Empty TOML with just [lint] section
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
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
[lint]
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
[lint]
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
[lint]
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
[lint]
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
[lint]
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
fn test_toml_select_rules_with_group() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // TOML that only selects any_is_na rule
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
select = ["any_is_na", "SUSP"]
"#,
    )?;

    let test_path = "test.R";
    let test_contents = "
any(is.na(x))
any(duplicated(x))
!all.equal(x, y)
";
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
[lint]
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
[lint]
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
[lint]
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
[lint]
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
[lint]
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
[lint]
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
[lint]
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
[lint
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
[lint]
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
[lint]
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
[lint]
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

#[test]
fn test_default_exclude_works() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // "default-exclude" is true by default
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
"#,
    )?;

    // This file is in the builtin list of excluded patterns
    let test_path = "cpp11.R";
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

    // "default-exclude" specified by the user
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
default-exclude = false
"#,
    )?;

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
fn test_default_exclude_wrong_values() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // "default-exclude" is true by default
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
default-exclude = 1
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

    // "default-exclude" specified by the user
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
default-exclude = ["a"]
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
fn test_exclude_single_file() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
exclude = ["excluded.R"]
"#,
    )?;

    // File that should be excluded
    let excluded_path = "excluded.R";
    let excluded_contents = "any(is.na(x))";
    std::fs::write(directory.join(excluded_path), excluded_contents)?;

    // File that should be checked
    let included_path = "included.R";
    let included_contents = "any(is.na(y))";
    std::fs::write(directory.join(included_path), included_contents)?;

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
fn test_exclude_directory() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
exclude = ["excluded_dir/"]
"#,
    )?;

    // Create excluded directory with files
    std::fs::create_dir(directory.join("excluded_dir"))?;
    std::fs::write(directory.join("excluded_dir/file.R"), "any(is.na(x))")?;

    // Create included file
    std::fs::write(directory.join("included.R"), "any(is.na(y))")?;

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
fn test_exclude_glob_pattern() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
exclude = ["test-*.R"]
"#,
    )?;

    // These two should be excluded
    std::fs::write(directory.join("test-one.R"), "any(is.na(x))")?;
    std::fs::write(directory.join("test-two.R"), "any(is.na(y))")?;
    // This one should be included
    std::fs::write(directory.join("normal.R"), "any(is.na(z))")?;

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
fn test_exclude_multiple_patterns() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
exclude = ["excluded.R", "temp/", "*.tmp.R"]
"#,
    )?;

    // Files that should be excluded
    std::fs::write(directory.join("excluded.R"), "any(is.na(a))")?;
    std::fs::create_dir(directory.join("temp"))?;
    std::fs::write(directory.join("temp/file.R"), "any(is.na(b))")?;
    std::fs::write(directory.join("test.tmp.R"), "any(is.na(c))")?;

    // File that should be included
    std::fs::write(directory.join("included.R"), "any(is.na(d))")?;

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
fn test_exclude_with_default_exclude_false() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
default-exclude = false
exclude = ["custom_exclude.R"]
"#,
    )?;

    // Should be included because default-exclude is false
    std::fs::write(directory.join("cpp11.R"), "any(is.na(x))")?;

    // Should be excluded by custom pattern
    std::fs::write(directory.join("custom_exclude.R"), "any(is.na(y))")?;

    std::fs::write(directory.join("normal.R"), "any(is.na(z))")?;

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
fn test_exclude_nested_directory_pattern() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
exclude = ["**/test/**"]
"#,
    )?;

    // Create nested test directories that should be excluded
    std::fs::create_dir_all(directory.join("src/test"))?;
    std::fs::write(directory.join("src/test/file.R"), "any(is.na(x))")?;

    std::fs::create_dir_all(directory.join("lib/test/deep"))?;
    std::fs::write(directory.join("lib/test/deep/file.R"), "any(is.na(y))")?;

    // Create files that should be included
    std::fs::create_dir(directory.join("other"))?;
    std::fs::write(directory.join("other/main.R"), "any(is.na(z))")?;

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
fn test_exclude_empty_array() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
exclude = []
"#,
    )?;

    std::fs::write(directory.join("test.R"), "any(is.na(x))")?;

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
fn test_exclude_wrong_values() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
exclude = true
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

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
exclude = 1
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

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
exclude = ["a", 1]
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
