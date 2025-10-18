use std::process::Command;
use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_nolint_leading_comment() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
# nolint
any(is.na(x))
any(is.na(x)) # nolint
",
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
fn test_nolint_not_applied_when_comment_inside_node() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
any(
  # nolint
  is.na(x)
)
",
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
fn test_nolint_with_specific_rules() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    std::fs::write(
        directory.join(test_path),
        "
# nolint: any_is_na
any(is.na(x))

# nolint: class_equals, any_is_na
any(is.na(x))

# compatibility with lintr
# nolint: any_is_na_linter
any(is.na(x))
# nolint: class_equals_linter, any_is_na_linter
any(is.na(x))

# nolint: any_duplicated
any(is.na(x))
",
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

// #[test]
// fn test_nolint_start_end() -> anyhow::Result<()> {
//     let directory = TempDir::new()?;
//     let directory = directory.path();

//     let test_path = "test.R";
//     std::fs::write(
//         directory.join(test_path),
//         "
// # nolint: any_is_na
// any(is.na(x))

// # nolint: class_equals, any_is_na
// any(is.na(x))

// # compatibility with lintr
// # nolint: any_is_na_linter
// any(is.na(x))
// # nolint: class_equals_linter, any_is_na_linter
// any(is.na(x))

// # nolint: any_duplicated
// any(is.na(x))
// ",
//     )?;

//     insta::assert_snapshot!(
//         &mut Command::new(binary_path())
//             .current_dir(directory)
//             .arg("check")
//             .arg(".")
//             .run()
//             .normalize_os_executable_name()
//     );

//     Ok(())
// }
