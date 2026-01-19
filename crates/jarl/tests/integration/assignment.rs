use std::process::Command;
use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_assignment_from_cli() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
x = 1
y <- 2
3 -> z
";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .arg("--assignment")
            .arg("<-")
            .run()
            .normalize_os_executable_name()
    );

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .arg("--assignment")
            .arg("=")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_assignment_from_toml() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
x = 1
y <- 2
3 -> z
";
    std::fs::write(directory.join(test_path), test_contents)?;
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
assignment = "<-"
"#,
    )?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .run()
            .normalize_os_executable_name()
    );

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
assignment = "="
"#,
    )?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_assignment_cli_overrides_toml() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
x = 1
y <- 2
3 -> z
";
    std::fs::write(directory.join(test_path), test_contents)?;
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
assignment = "<-"
"#,
    )?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .arg("--assignment")
            .arg("=")
            .run()
            .normalize_os_executable_name()
    );
    Ok(())
}

#[test]
fn test_assignment_wrong_value_from_cli() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
x = 1
y <- 2
3 -> z
";
    std::fs::write(directory.join(test_path), test_contents)?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .arg("--assignment")
            .arg("foo")
            .run()
            .normalize_os_executable_name()
    );

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .arg("--assignment")
            .arg("1")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_assignment_wrong_value_from_toml() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
x = 1
y <- 2
3 -> z
";
    std::fs::write(directory.join(test_path), test_contents)?;
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
assignment = "foo"
"#,
    )?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
    );

    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
assignment = 1
"#,
    )?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("assignment")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths()
    );

    Ok(())
}
