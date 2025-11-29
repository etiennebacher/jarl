use std::process::Command;
use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_no_default_exclude() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "cpp11.R";
    let test_contents = "x = 1";

    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--no-default-exclude")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}
#[test]
fn test_no_default_exclude_overrides_toml() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "cpp11.R";
    let test_contents = "x = 1";

    std::fs::write(directory.join(test_path), test_contents)?;
    std::fs::write(
        directory.join("jarl.toml"),
        r#"
[lint]
default-exclude = true
"#,
    )?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--no-default-exclude")
            .run()
            .normalize_os_executable_name()
    );
    Ok(())
}
