use std::process::Command;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_help() {
    insta::assert_snapshot!(
        Command::new(binary_path())
            .arg("help")
            .run()
            .normalize_os_executable_name()
    );
    insta::assert_snapshot!(
        Command::new(binary_path())
            .arg("--help")
            .run()
            .normalize_os_executable_name()
    );
    insta::assert_snapshot!(
        Command::new(binary_path())
            .arg("-h")
            .run()
            .normalize_os_executable_name()
    );
}
