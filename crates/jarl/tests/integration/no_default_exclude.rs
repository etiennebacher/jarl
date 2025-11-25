use std::process::Command;
use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_no_default_exclude() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "cpp11.R";
    let test_contents = "
x = 1
y <- 2
3 -> z
";
    std::fs::create_dir_all(directory.join("demos"))?;
    std::fs::write(directory.join(test_path), test_contents)?;

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
