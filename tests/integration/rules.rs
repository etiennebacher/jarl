use std::process::Command;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_one_non_existing_rule() -> anyhow::Result<()> {
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .arg("--rules")
            .arg("foo")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_several_non_existing_rules() -> anyhow::Result<()> {
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .arg("--rules")
            .arg("foo,any_is_na,barbaz")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}
