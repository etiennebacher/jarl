use std::process::Command;
use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_no_default_exclude() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "cpp11.R";
    let test_contents = "any(is.na(x))";

    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
                        &mut Command::new(binary_path())
                            .current_dir(directory)
                            .arg("check")
                            .arg(".")
                            .run()
                            .normalize_os_executable_name(),
                        @r"
success: true
exit_code: 0
----- stdout -----
Warning: No R files found under the given path(s).

----- stderr -----
"
                    );

    insta::assert_snapshot!(
                        &mut Command::new(binary_path())
                            .current_dir(directory)
                            .arg("check")
                            .arg(".")
                            .arg("--no-default-exclude")
                            .run()
                            .normalize_os_executable_name(),
                        @r"
success: false
exit_code: 1
----- stdout -----
warning: any_is_na
 --> cpp11.R:1:1
  |
1 | any(is.na(x))
  | ------------- `any(is.na(...))` is inefficient.
  |
  = help: Use `anyNA(...)` instead.

Found 1 error.
1 fixable with the `--fix` option.

----- stderr -----
"
                    );

    Ok(())
}
#[test]
fn test_no_default_exclude_overrides_toml() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "cpp11.R";
    let test_contents = "any(is.na(x))";

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
                            .normalize_os_executable_name(),
                        @r"
success: false
exit_code: 1
----- stdout -----
warning: any_is_na
 --> cpp11.R:1:1
  |
1 | any(is.na(x))
  | ------------- `any(is.na(...))` is inefficient.
  |
  = help: Use `anyNA(...)` instead.

Found 1 error.
1 fixable with the `--fix` option.

----- stderr -----
"
                    );
    Ok(())
}
