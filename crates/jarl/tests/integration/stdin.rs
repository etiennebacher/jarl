use crate::helpers::{CliTest, Output};
use std::io::Write;
use std::process::Stdio;

/// Run `jarl check <args>` with `code` piped to stdin.
fn check_stdin(case: &CliTest, code: &str, args: &[&str]) -> anyhow::Result<Output> {
    let mut command = case.command();
    command
        .arg("check")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let arguments = command
        .get_args()
        .map(|x| x.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ");

    let mut child = command.spawn()?;
    child.stdin.take().unwrap().write_all(code.as_bytes())?;
    let output = child.wait_with_output()?;

    Ok(Output {
        status: output.status,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        arguments,
    }
    .normalize_os_executable_name())
}

#[test]
fn test_stdin_uses_virtual_path_and_config() -> anyhow::Result<()> {
    let case = CliTest::with_file("jarl.toml", "[lint]\nignore = [\"any_is_na\"]\n")?;

    // The buffer is linted, not the file on disk, and `jarl.toml` applies.
    insta::assert_snapshot!(check_stdin(
        &case,
        "any(is.na(x))\nany(duplicated(x))\n",
        &["--stdin-file-path", "R/foo.R"],
    )?, @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_duplicated
     --> R/foo.R:2:1
      |
    2 | any(duplicated(x))
      | ------------------ `any(duplicated(...))` is inefficient.
      |
      = help: Use `anyDuplicated(...) > 0` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    ");
    Ok(())
}

#[test]
fn test_stdin_virtual_file_does_not_need_to_exist() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    insta::assert_snapshot!(
        check_stdin(&case, "any(is.na(x))\n", &["--stdin-file-path", "nope/new.R"])?,
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> nope/new.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );
    Ok(())
}

#[test]
fn test_stdin_filename_rejects_fix() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    insta::assert_snapshot!(
        check_stdin(&case, "x <- 1\n", &["--stdin-file-path", "a.R", "--fix"])?,
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: the argument '--stdin-file-path <PATH>' cannot be used with '--fix'

    Usage: jarl check --stdin-file-path <PATH> [FILES]...

    For more information, try '--help'.
    "
    );
    Ok(())
}

#[test]
fn test_stdin_rejects_unsafe_fixes() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    insta::assert_snapshot!(
        check_stdin(&case, "x <- 1\n", &["--stdin-file-path", "a.R", "--unsafe-fixes"])?,
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: the argument '--stdin-file-path <PATH>' cannot be used with '--unsafe-fixes'

    Usage: jarl check --stdin-file-path <PATH> [FILES]...

    For more information, try '--help'.
    "
    );
    Ok(())
}

#[test]
fn test_stdin_rejects_fix_only() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    insta::assert_snapshot!(
        check_stdin(&case, "x <- 1\n", &["--stdin-file-path", "a.R", "--fix-only"])?,
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: the argument '--stdin-file-path <PATH>' cannot be used with '--fix-only'

    Usage: jarl check --stdin-file-path <PATH> [FILES]...

    For more information, try '--help'.
    "
    );
    Ok(())
}

#[test]
fn test_stdin_rejects_add_jarl_ignore() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    insta::assert_snapshot!(
        check_stdin(&case, "x <- 1\n", &["--stdin-file-path", "a.R", "--add-jarl-ignore"])?,
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: the argument '--stdin-file-path <PATH>' cannot be used with '--add-jarl-ignore[=<REASON>]'

    Usage: jarl check --stdin-file-path <PATH> [FILES]...

    For more information, try '--help'.
    "
    );
    Ok(())
}

#[test]
fn test_stdin_file_path_conflicts_with_files() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    insta::assert_snapshot!(
        check_stdin(&case, "x <- 1\n", &["folder/", "--stdin-file-path", "a.R"])?,
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: the argument '[FILES]...' cannot be used with '--stdin-file-path <PATH>'

    Usage: jarl check <FILES>...

    For more information, try '--help'.
    "
    );
    Ok(())
}

#[test]
fn test_stdin_respects_exclude_in_config() -> anyhow::Result<()> {
    let case = CliTest::with_file("jarl.toml", "[lint]\nexclude = [\"R/skip.R\"]\n")?;
    insta::assert_snapshot!(
        check_stdin(&case, "any(is.na(x))\n", &["--stdin-file-path", "R/skip.R"])?,
        @"

    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    "
    );
    insta::assert_snapshot!(
        check_stdin(&case, "any(is.na(x))\n", &["--stdin-file-path", "R/keep.R"])?,
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/keep.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );
    Ok(())
}

#[test]
fn test_stdin_applies_default_exclude_without_config() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    insta::assert_snapshot!(
        check_stdin(&case, "any(is.na(x))\n", &["--stdin-file-path", "renv/a.R"])?,
        @"

    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    "
    );
    insta::assert_snapshot!(
        check_stdin(
            &case,
            "any(is.na(x))\n",
            &["--stdin-file-path", "renv/a.R", "--no-default-exclude"],
        )?,
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> renv/a.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );
    Ok(())
}

#[test]
fn test_stdin_snippet_comes_from_stdin_not_disk() -> anyhow::Result<()> {
    let case = CliTest::with_file("a.R", "# on disk\n")?;
    insta::assert_snapshot!(
        check_stdin(&case, "any(is.na(x))\n", &["--stdin-file-path", "a.R"])?,
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> a.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );
    Ok(())
}

#[test]
fn test_stdin_file_path_cannot_be_repeated() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    insta::assert_snapshot!(
        check_stdin(
            &case,
            "any(is.na(x))\n",
            &["--stdin-file-path", "a.R", "--stdin-file-path", "b.R"],
        )?,
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: the argument '--stdin-file-path <PATH>' cannot be used multiple times

    Usage: jarl check [OPTIONS] [FILES]...

    For more information, try '--help'.
    "
    );
    Ok(())
}

#[test]
fn test_stdin_file_path_is_not_split_on_commas() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    insta::assert_snapshot!(
        check_stdin(&case, "any(is.na(x))\n", &["--stdin-file-path", "a.R,b.R"])?,
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> a.R,b.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );
    Ok(())
}
