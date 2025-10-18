use std::process::Command;

use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_must_pass_path() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_no_r_files() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_parsing_error() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let path = "test.R";
    std::fs::write(directory.join(path), "f <-")?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_parsing_error_for_some_files() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let path = "test.R";
    std::fs::write(directory.join(path), "f <-")?;

    let path = "test2.R";
    std::fs::write(directory.join(path), "any(is.na(x))")?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_parsing_weird_raw_strings() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let path = "test.R";
    std::fs::write(
        directory.join(path),
        "c(r\"(abc(\\w+))\")\nr\"(c(\"\\dots\"))\"",
    )?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_parsing_braced_anonymous_function() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let path = "test.R";
    std::fs::write(directory.join(path), "{ a }(10)")?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_no_lints() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let path = "test.R";
    std::fs::write(directory.join(path), "any(x)")?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_one_lint() -> anyhow::Result<()> {
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
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_several_lints_one_file() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))\nany(duplicated(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_several_lints_several_files() -> anyhow::Result<()> {
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
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_not_all_fixable_lints() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    let test_path_2 = "test2.R";
    let test_contents_2 = "list(x = 1, x = 2)";
    std::fs::write(directory.join(test_path_2), test_contents_2)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_corner_case() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "x %>% length()";
    std::fs::write(directory.join(test_path), test_contents)?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_fix_options() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    // File with 3 lints:
    // - any_is_na (has fix)
    // - class_equals (has unsafe fix)
    // - duplicated_arguments (has no fix)
    let test_path = "test.R";
    let test_contents = "any(is.na(x))\nclass(x) == 'foo'\nlist(x = 1, x = 2)";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--fix")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    std::fs::write(directory.join(test_path), test_contents)?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--fix")
            .arg("--unsafe-fixes")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    std::fs::write(directory.join(test_path), test_contents)?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--fix")
            .arg("--unsafe-fixes")
            .arg("--fix-only")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    std::fs::write(directory.join(test_path), test_contents)?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--fix")
            .arg("--fix-only")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    std::fs::write(directory.join(test_path), test_contents)?;
    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--unsafe-fixes")
            .arg("--fix-only")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_safe_and_unsafe_lints() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    let test_path_2 = "test2.R";
    let test_contents_2 = "class(x) == 'a'";
    std::fs::write(directory.join(test_path_2), test_contents_2)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}

#[test]
fn test_newline_character_in_string() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "print(\"hi there\\n\")\nany(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
        &mut Command::new(binary_path())
            .current_dir(directory)
            .arg("check")
            .arg(".")
            .arg("--allow-no-vcs")
            .run()
            .normalize_os_executable_name()
    );

    Ok(())
}
