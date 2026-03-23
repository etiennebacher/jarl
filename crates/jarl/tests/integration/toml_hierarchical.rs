use crate::helpers::{CliTest, CommandExt};

#[test]
fn test_look_for_toml_in_parent_directories() -> anyhow::Result<()> {
    let case = CliTest::new()?;

    // Can't create a parent of tempdir, so create a "subdir" that mimicks the
    // current project directory and use "root_dir" as a parent directory.
    case.write_file("subdir/test.R", "any(is.na(x))\nany(duplicated(x))")?;

    // At this point, there is no TOML to detect in the current or parent
    // directory, so both violations should be reported.
    insta::assert_snapshot!(
        &mut case
            .command()
            .current_dir(case.root().join("subdir"))
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

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


    ── Summary ──────────────────────────────────────
    Found 2 errors.
    2 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    // Place a TOML in the root directory, which is the parent directory of
    // the current project.
    case.write_file(
        "jarl.toml",
        r#"
[lint]
ignore = ["any_is_na"]
"#,
    )?;

    // Now, this should find the TOML in the parent directory and report only
    // one violation.
    insta::assert_snapshot!(
        &mut case
            .command()
            .current_dir(case.root().join("subdir"))
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_duplicated
     --> test.R:2:1
      |
    2 | any(duplicated(x))
      | ------------------ `any(duplicated(...))` is inefficient.
      |
      = help: Use `anyDuplicated(...) > 0` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ── Notes ────────────────────────────────────────
    Used '[TEMP_DIR]/jarl.toml'

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_nearest_toml_takes_precedence() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("subdir/test.R", "any(is.na(x))\nany(duplicated(x))"),
        (
            "jarl.toml",
            r#"
[lint]
ignore = ["any_is_na"]
"#,
        ),
        (
            "subdir/jarl.toml",
            r#"
[lint]
ignore = ["any_duplicated"]
"#,
        ),
    ])?;

    // This sould ignore any_duplicated because it's in the closest TOML.
    insta::assert_snapshot!(
        &mut case
            .command()
            .current_dir(case.root().join("subdir"))
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

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


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_no_toml_uses_defaults() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "any(is.na(x))\nany(duplicated(x))")?;

    // Should use default settings (both lints fire)
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

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


    ── Summary ──────────────────────────────────────
    Found 2 errors.
    2 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_explicit_file_finds_parent_toml() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("project/script.R", "any(is.na(x))\nany(duplicated(x))"),
        (
            "project/jarl.toml",
            r#"
[lint]
ignore = ["any_duplicated"]
"#,
        ),
    ])?;

    // Run from root but specify file path explicitly
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("project/script.R")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> project/script.R:1:1
      |
    1 | any(is.na(x))
      | ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ── Notes ────────────────────────────────────────
    Used '[TEMP_DIR]/project/jarl.toml'

    ----- stderr -----
    "
    );

    Ok(())
}
