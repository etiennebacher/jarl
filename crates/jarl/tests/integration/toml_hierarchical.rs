use std::process::Command;

use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_look_for_toml_in_parent_directories() -> anyhow::Result<()> {
    let root_dir = TempDir::new()?;
    let root_path = root_dir.path();

    // Can't create a parent of tempdir, so create a "subdir" that mimicks the
    // current project directory and use "root_dir" as a parent directory.
    let subdir = root_path.join("subdir");
    std::fs::create_dir_all(&subdir)?;

    // Create an R file in "subdir"
    let test_file = subdir.join("test.R");
    let test_contents = "any(is.na(x))\nany(duplicated(x))";
    std::fs::write(&test_file, test_contents)?;

    // At this point, there is no TOML to detect in the current or parent
    // directory, so both violations should be reported.
    insta::assert_snapshot!(
        "no_toml_anywhere",
        &mut Command::new(binary_path())
            .current_dir(&subdir)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
    );

    // Place a TOML in the root directory, which is the parent directory of
    // the current project.
    std::fs::write(
        root_path.join("jarl.toml"),
        r#"
[lint]
ignore = ["any_is_na"]
"#,
    )?;

    // Now, this should find the TOML in the parent directory and report only
    // one violation.
    insta::assert_snapshot!(
        "parent_toml_found",
        &mut Command::new(binary_path())
            .current_dir(&subdir)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
    );

    Ok(())
}

#[test]
fn test_nearest_toml_takes_precedence() -> anyhow::Result<()> {
    let root_dir = TempDir::new()?;
    let root_path = root_dir.path();

    // Can't create a parent of tempdir, so create a "subdir" that mimicks the
    // current project directory and use "root_dir" as a parent directory.
    let subdir = root_path.join("subdir");
    std::fs::create_dir_all(&subdir)?;

    // Create an R file in "subdir"
    let test_file = subdir.join("test.R");
    let test_contents = "any(is.na(x))\nany(duplicated(x))";
    std::fs::write(&test_file, test_contents)?;

    // Place a TOML in the root directory, which is the parent directory of
    // the current project.
    std::fs::write(
        root_path.join("jarl.toml"),
        r#"
[lint]
ignore = ["any_is_na"]
"#,
    )?;

    // Place another TOML in the subdir directory, which is the current directory.
    // This one should be found first and therefore should take precedence.
    std::fs::write(
        subdir.join("jarl.toml"),
        r#"
[lint]
ignore = ["any_duplicated"]
"#,
    )?;

    // This sould ignore any_duplicated because it's in the closest TOML.
    insta::assert_snapshot!(
        "should_ignore_any_duplicated",
        &mut Command::new(binary_path())
            .current_dir(subdir)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
    );

    Ok(())
}

#[test]
fn test_no_toml_uses_defaults() -> anyhow::Result<()> {
    let root_dir = TempDir::new()?;
    let root_path = root_dir.path();

    // Create R file with no jarl.toml anywhere
    let test_file = root_path.join("test.R");
    let test_contents = "any(is.na(x))\nany(duplicated(x))";
    std::fs::write(&test_file, test_contents)?;

    // Should use default settings (both lints fire)
    insta::assert_snapshot!(
        "no_config_uses_defaults",
        &mut Command::new(binary_path())
            .current_dir(root_path)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
    );

    Ok(())
}

#[test]
fn test_explicit_file_finds_parent_toml() -> anyhow::Result<()> {
    let root_dir = TempDir::new()?;
    let root_path = root_dir.path();

    // Create nested structure
    let subdir = root_path.join("project");
    std::fs::create_dir_all(&subdir)?;

    // Create file in subdirectory
    let test_file = subdir.join("script.R");
    std::fs::write(&test_file, "any(is.na(x))\nany(duplicated(x))")?;

    // Place TOML in subdirectory
    std::fs::write(
        subdir.join("jarl.toml"),
        r#"
[lint]
ignore = ["any_duplicated"]
"#,
    )?;

    // Run from root but specify file path explicitly
    insta::assert_snapshot!(
        "explicit_path_finds_parent_config",
        &mut Command::new(binary_path())
            .current_dir(root_path)
            .arg("check")
            .arg("project/script.R")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
    );

    Ok(())
}
