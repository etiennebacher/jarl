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
            .normalize_os_executable_name(),
        @r#"
success: false
exit_code: 2
----- stdout -----

----- stderr -----
Check a set of files or directories

Usage: jarl check [OPTIONS] <FILES>...

Arguments:
  <FILES>...  List of files or directories to check or fix lints, for example `jarl check .`.

Options:
  -f, --fix                            Automatically fix issues detected by the linter.
  -u, --unsafe-fixes                   Include fixes that may not retain the original intent of the  code.
      --fix-only                       Apply fixes to resolve lint violations, but don't report on leftover violations. Implies `--fix`.
      --allow-dirty                    Apply fixes even if the Git branch is not clean, meaning that there are uncommitted files.
      --allow-no-vcs                   Apply fixes even if there is no version control system.
  -s, --select <SELECT>                Names of rules to include, separated by a comma (no spaces). This also accepts names of groups of rules, such as "PERF". [default: ]
  -e, --extend-select <EXTEND_SELECT>  Like `--select` but adds additional rules in addition to those already specified. [default: ]
  -i, --ignore <IGNORE>                Names of rules to exclude, separated by a comma (no spaces). This also accepts names of groups of rules, such as "PERF". [default: ]
  -w, --with-timing                    Show the time taken by the function.
  -m, --min-r-version <MIN_R_VERSION>  The mimimum R version to be used by the linter. Some rules only work starting from a specific version.
      --output-format <OUTPUT_FORMAT>  Output serialization format for violations. [default: full] [possible values: full, concise, github, json]
      --assignment <ASSIGNMENT>        [DEPRECATED: use `[lint.assignment]` in jarl.toml] Assignment operator to use, can be either `<-` or `=`.
      --no-default-exclude             Do not apply the default set of file patterns that should be excluded.
      --statistics                     Show counts for every rule with at least one violation.
      --add-jarl-ignore[=<REASON>]     Automatically insert a `# jarl-ignore` comment to suppress all violations.
                                       The default reason can be customized with `--add-jarl-ignore="my_reason"`.
  -h, --help                           Print help (see more with '--help')

Global options:
      --log-level <LOG_LEVEL>  The log level. One of: `error`, `warn`, `info`, `debug`, or `trace`. Defaults to `warn`
"#
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
                                .normalize_os_executable_name(),
                            @r"
success: true
exit_code: 0
----- stdout -----
Warning: No R files found under the given path(s).

----- stderr -----
"
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
                                .normalize_os_executable_name(),
                            @r"
success: false
exit_code: 255
----- stdout -----

----- stderr -----
Error: Failed to parse test.R due to syntax errors.
"
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
                                .normalize_os_executable_name(),
                            @r"
success: false
exit_code: 255
----- stdout -----
warning: any_is_na
 --> test2.R:1:1
  |
1 | any(is.na(x))
  | ------------- `any(is.na(...))` is inefficient.
  |
  = help: Use `anyNA(...)` instead.

Found 1 error.
1 fixable with the `--fix` option.

----- stderr -----
Error: Failed to parse test.R due to syntax errors.

"
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
                                .normalize_os_executable_name(),
                            @r"
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
                                .normalize_os_executable_name(),
                            @r"
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
                                .normalize_os_executable_name(),
                            @r"
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
                                .normalize_os_executable_name(),
                            @r"
success: false
exit_code: 1
----- stdout -----
warning: any_is_na
 --> test.R:1:1
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
                                .normalize_os_executable_name(),
                            @r"
success: false
exit_code: 1
----- stdout -----
warning: any_is_na
 --> test.R:1:1
  |
1 | any(is.na(x))
  | ------------- `any(is.na(...))` is inefficient.
  |
  = help: Use `anyNA(...)` instead.

warning: any_duplicated
 --> test.R:2:1
  |
2 | any(duplicated(x))
  | ------------------ `any(duplicated(...))` is inefficient.
  |
  = help: Use `anyDuplicated(...) > 0` instead.

Found 2 errors.
2 fixable with the `--fix` option.

----- stderr -----
"
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
                                .normalize_os_executable_name(),
                            @r"
success: false
exit_code: 1
----- stdout -----
warning: any_is_na
 --> test.R:1:1
  |
1 | any(is.na(x))
  | ------------- `any(is.na(...))` is inefficient.
  |
  = help: Use `anyNA(...)` instead.

warning: any_duplicated
 --> test2.R:1:1
  |
1 | any(duplicated(x))
  | ------------------ `any(duplicated(...))` is inefficient.
  |
  = help: Use `anyDuplicated(...) > 0` instead.

Found 2 errors.
2 fixable with the `--fix` option.

----- stderr -----
"
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
            .normalize_os_executable_name(),
        @r#"
success: false
exit_code: 1
----- stdout -----
warning: any_is_na
 --> test.R:1:1
  |
1 | any(is.na(x))
  | ------------- `any(is.na(...))` is inefficient.
  |
  = help: Use `anyNA(...)` instead.

warning: duplicated_arguments
 --> test2.R:1:1
  |
1 | list(x = 1, x = 2)
  | ------------------ Avoid duplicate arguments in function calls. Duplicated argument(s): "x".
  |

Found 2 errors.
1 fixable with the `--fix` option.

----- stderr -----
"#
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
                                .normalize_os_executable_name(),
                            @r"
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
            .normalize_os_executable_name(),
        @r#"
success: false
exit_code: 1
----- stdout -----
warning: duplicated_arguments
 --> test.R:3:1
  |
3 | list(x = 1, x = 2)
  | ------------------ Avoid duplicate arguments in function calls. Duplicated argument(s): "x".
  |

Found 1 error.

----- stderr -----
"#
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
            .normalize_os_executable_name(),
        @r#"
success: false
exit_code: 1
----- stdout -----
warning: duplicated_arguments
 --> test.R:3:1
  |
3 | list(x = 1, x = 2)
  | ------------------ Avoid duplicate arguments in function calls. Duplicated argument(s): "x".
  |

Found 1 error.

----- stderr -----
"#
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
                                .normalize_os_executable_name(),
                            @r"
success: true
exit_code: 0
----- stdout -----
All checks passed!

----- stderr -----
"
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
                                .normalize_os_executable_name(),
                            @r"
success: true
exit_code: 0
----- stdout -----
All checks passed!

----- stderr -----
"
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
                                .normalize_os_executable_name(),
                            @r"
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
fn test_safe_and_unsafe_lints() -> anyhow::Result<()> {
    let directory = TempDir::new()?;
    let directory = directory.path();

    let test_path = "test.R";
    let test_contents = "any(is.na(x))";
    std::fs::write(directory.join(test_path), test_contents)?;

    let test_path_2 = "test2.R";
    let test_contents_2 = "!all.equal(x, y)";
    std::fs::write(directory.join(test_path_2), test_contents_2)?;

    insta::assert_snapshot!(
                            &mut Command::new(binary_path())
                                .current_dir(directory)
                                .arg("check")
                                .arg(".")
                                .arg("--allow-no-vcs")
                                .run()
                                .normalize_os_executable_name(),
                            @r"
success: false
exit_code: 1
----- stdout -----
warning: any_is_na
 --> test.R:1:1
  |
1 | any(is.na(x))
  | ------------- `any(is.na(...))` is inefficient.
  |
  = help: Use `anyNA(...)` instead.

warning: all_equal
 --> test2.R:1:1
  |
1 | !all.equal(x, y)
  | ---------------- If `all.equal()` is false, it will return a string and not `FALSE`.
  |
  = help: Wrap `all.equal()` in `isTRUE()`, or replace it by `identical()` if no tolerance is required.

Found 2 errors.
1 fixable with the `--fix` option (1 hidden fix can be enabled with the `--unsafe-fixes` option).

----- stderr -----
"
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
                                .normalize_os_executable_name(),
                            @r"
success: false
exit_code: 1
----- stdout -----
warning: any_is_na
 --> test.R:2:1
  |
2 | any(is.na(x))
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
