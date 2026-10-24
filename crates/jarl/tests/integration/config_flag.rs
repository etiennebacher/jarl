use crate::helpers::{CliTest, CommandExt};

#[test]
fn test_config_flag_overrides_discovery() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("subdir/test.R", "any(is.na(x))\nany(duplicated(x))"),
        (
            "config/settings.toml",
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

    // The config given on the command line wins over the `jarl.toml` that
    // discovery would have found in the current directory, so `any_is_na` is
    // the rule that is ignored here.
    insta::assert_snapshot!(
        &mut case
            .command()
            .current_dir(case.root().join("subdir"))
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("../config/settings.toml")
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

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_config_flag_applies_to_nested_directories() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
        ("nested/test.R", "any(is.na(x))\nany(duplicated(x))"),
        (
            "config/settings.toml",
            r#"
[lint]
ignore = ["any_is_na"]
"#,
        ),
        (
            "nested/jarl.toml",
            r#"
[lint]
ignore = ["any_duplicated"]
"#,
        ),
    ])?;

    // A `jarl.toml` found below the checked path is ignored too: only the file
    // named by `--config` is used.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("config/settings.toml")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_duplicated
     --> nested/test.R:2:1
      |
    2 | any(duplicated(x))
      | ------------------ `any(duplicated(...))` is inefficient.
      |
      = help: Use `anyDuplicated(...) > 0` instead.

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
fn test_config_flag_outside_the_tree_keeps_per_file_ignores() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("R/test.R", "any(is.na(x))\nany(duplicated(x))"),
        (
            "config/settings.toml",
            r#"
[lint.per-file-ignores]
"R/" = ["any_is_na"]
"#,
        ),
    ])?;

    // The config lives outside the linted tree here. Its relative patterns must still be
    // anchored where the files are, or `per-file-ignores` silently stops applying.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("R")
            .arg("--config")
            .arg("config/settings.toml")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_duplicated
     --> R/test.R:2:1
      |
    2 | any(duplicated(x))
      | ------------------ `any(duplicated(...))` is inefficient.
      |
      = help: Use `anyDuplicated(...) > 0` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_config_flag_outside_the_tree_keeps_exclude() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("test.R", "any(is.na(x))"),
        ("renv/ignored.R", "any(is.na(x))"),
        ("config/settings.toml", "[lint]\n"),
    ])?;

    // A path given with `--stdin-file-path` is checked against the same exclusions as a
    // file found on disk, which only works if the config is anchored at the directory the
    // command runs from.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("--stdin-file-path")
            .arg("renv/ignored.R")
            .arg("--config")
            .arg("config/settings.toml")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_config_flag_given_as_a_bare_filename() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
        ("settings.toml", "[lint]\nignore = [\"any_is_na\"]\n"),
    ])?;

    // No directory component: the file is anchored where the command runs from, and the
    // config still wins over discovery.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("settings.toml")
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

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_config_flag_missing_file() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "any(is.na(x))")?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("nope.toml")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Configuration file not found: [TEMP_DIR]/nope.toml
    "
    );

    Ok(())
}

#[test]
fn test_config_flag_rejects_directory() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("test.R", "any(is.na(x))"),
        ("config/jarl.toml", "[lint]\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("config")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: `--config` expects a file, but [TEMP_DIR]/config is not a file
    "
    );

    Ok(())
}
