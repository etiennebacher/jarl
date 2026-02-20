use std::process::Command;

use tempfile::TempDir;

use crate::helpers::CommandExt;
use crate::helpers::binary_path;

#[test]
fn test_stats() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
";
    std::fs::write(directory.join(test_path), test_contents)?;

    let test_path_2 = "test2.R";
    let test_contents_2 = "mean(x <- 1)";
    std::fs::write(directory.join(test_path_2), test_contents_2)?;

    insta::assert_snapshot!(
                            &mut Command::new(binary_path())
                                .current_dir(directory)
                                .arg("check")
                                .arg(".")
                                .arg("--statistics")
                                .run()
                                .normalize_os_executable_name(),
                            @"
success: false
exit_code: 1
----- stdout -----
   12 [*] any_is_na
    1 [ ] implicit_assignment

Rules with `[*]` have an automatic fix.

----- stderr -----
"
                        );

    Ok(())
}

#[test]
fn test_stats_no_violation() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "x <- 1";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
                            &mut Command::new(binary_path())
                                .current_dir(directory)
                                .arg("check")
                                .arg(".")
                                .arg("--statistics")
                                .run()
                                .normalize_os_executable_name(),
                            @"
success: true
exit_code: 0
----- stdout -----
All checks passed!

----- stderr -----
"
                        );

    Ok(())
}

#[test]
fn test_hint_stats_arg() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
                            &mut Command::new(binary_path())
                                .current_dir(directory)
                                .arg("check")
                                .arg(".")
                                .arg("--output-format")
                                .arg("concise")
                                .run()
                                .normalize_os_executable_name(),
                            @"
success: false
exit_code: 1
----- stdout -----
test.R [2:2] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [3:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [4:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [5:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [6:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [7:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [8:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [9:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [10:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [11:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [12:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [13:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [14:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [15:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [16:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [17:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [18:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.

Found 17 errors.
17 fixable with the `--fix` option.

More than 15 errors reported, use `--statistics` to get the count by rule.

----- stderr -----
"
                        );

    Ok(())
}

#[test]
fn test_hint_stats_arg_with_envvar() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
any(is.na(x))
";
    std::fs::write(directory.join(test_path), test_contents)?;

    insta::assert_snapshot!(
                            &mut Command::new(binary_path())
                                .current_dir(directory)
                                .arg("check")
                                .arg(".")
                                .arg("--output-format")
                                .arg("concise")
                                .env("JARL_N_VIOLATIONS_HINT_STAT", "25")
                                .run()
                                .normalize_os_executable_name(),
                            @"
success: false
exit_code: 1
----- stdout -----
test.R [2:2] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [3:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [4:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [5:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [6:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [7:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [8:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [9:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [10:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [11:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [12:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [13:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [14:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [15:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [16:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [17:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.
test.R [18:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.

Found 17 errors.
17 fixable with the `--fix` option.

----- stderr -----
"
                        );

    Ok(())
}
