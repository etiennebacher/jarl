use std::process::Command;
use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_min_r_version_from_cli_only() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "grep('a', x, value = TRUE)";
    std::fs::write(directory.join(test_path), test_contents)?;

    // grepv() rule only exists for R >= 4.5.

    // By default, if we don't know the min R version, we disable rules that
    // only exist starting from a specific version.
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    // This should not report a lint (the project could be using 4.4.0 so
    // grepv() wouldn't exist).
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg(".")
            .arg("--min-r-version")
            .arg("4.4.0")
            .run()
            .normalize_os_executable_name()
    );
    // This should report a lint.
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg(".")
            .arg("--min-r-version")
            .arg("4.6.0")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_min_r_version_from_description_only() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "grep('a', x, value = TRUE)";
    std::fs::write(directory.join(test_path), test_contents)?;

    // grepv() rule only exists for R >= 4.5.0

    // This should not report a lint (the project could be using 4.4.0 so
    // grepv() wouldn't exist).
    std::fs::write(
        directory.join("DESCRIPTION"),
        r#"Package: mypackage
Version: 1.0.0
Depends: R (>= 4.4.0), utils, stats"#,
    )?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    // This should report a lint.
    std::fs::write(
        directory.join("DESCRIPTION"),
        r#"Package: mypackage
Version: 1.0.0
Depends: R (>= 4.6.0), utils, stats"#,
    )?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg(".")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}
