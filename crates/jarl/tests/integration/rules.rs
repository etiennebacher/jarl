use std::process::Command;

use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_one_non_existing_selected_rule() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select-rules")
            .arg("foo")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_several_non_existing_selected_rules() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select-rules")
            .arg("foo,any_is_na,barbaz")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_one_non_existing_ignored_rule() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--ignore-rules")
            .arg("foo")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_several_non_existing_ignored_rules() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--ignore-rules")
            .arg("foo,any_is_na,barbaz")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_selected_and_ignored() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select-rules")
            .arg("any_is_na")
            .arg("--ignore-rules")
            .arg("any_is_na")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_correct_rule_selection_and_exclusion() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    let test_path_2 = "test2.R";
    let test_contents_2 = "any(duplicated(x))";
    std::fs::write(directory.join(test_path_2), test_contents_2)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select-rules")
            .arg("any_is_na")
            .arg("--ignore-rules")
            .arg("any_duplicated")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_select_rule_group() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
any(is.na(x))
!all.equal(x, y)
";
    std::fs::write(directory.join(test_path), test_contents)?;

    // Works with only group name
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select-rules")
            .arg("SUSP")
            .run()
            .normalize_os_executable_name()
    );

    // Can mix group name and rule name
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select-rules")
            .arg("any_is_na,SUSP")
            .run()
            .normalize_os_executable_name()
    );

    // Can mix group name and rule name that is part of the same group
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select-rules")
            .arg("all_equal,SUSP")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_ignore_rule_group() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
any(is.na(x))
!all.equal(x, y)
";
    std::fs::write(directory.join(test_path), test_contents)?;

    // Works with only group name
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--ignore-rules")
            .arg("SUSP")
            .run()
            .normalize_os_executable_name()
    );

    // Can mix group name and rule name
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--ignore-rules")
            .arg("any_is_na,SUSP")
            .run()
            .normalize_os_executable_name()
    );

    // Can mix group name and rule name that is part of the same group
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--ignore-rules")
            .arg("all_equal,SUSP")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_invalid_rule_group() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    // Works with only group name
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--ignore-rules")
            .arg("FOOBAR,SUSP")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_select_ignore_interaction_with_rule_group() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
any(is.na(x))
!all.equal(x, y)
";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select-rules")
            .arg("SUSP")
            .arg("--ignore-rules")
            .arg("SUSP")
            .run()
            .normalize_os_executable_name()
    );

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--select-rules")
            .arg("SUSP")
            .arg("--ignore-rules")
            .arg("PERF")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}
