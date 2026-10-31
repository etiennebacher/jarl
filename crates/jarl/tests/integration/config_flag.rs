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

#[test]
fn test_config_flag_accepts_inline_toml() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "any(is.na(x))\nany(duplicated(x))")?;

    // Ruff's `--config` also takes a TOML snippet, so a single option can be overridden
    // without writing a file.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("lint.ignore = [\"any_is_na\"]")
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
fn test_config_flag_repeated_values_are_merged() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
        ("skip/other.R", "any(duplicated(z))"),
        ("settings.toml", "[lint]\nexclude = [\"skip/\"]\n"),
    ])?;

    // The snippet overrides one key and leaves the rest of the file alone: `exclude` still
    // keeps `skip/other.R` out, and `ignore` is added by the second value.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("settings.toml")
            .arg("--config")
            .arg("lint.ignore = [\"any_is_na\"]")
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
fn test_config_flag_later_value_wins() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
        ("settings.toml", "[lint]\nignore = [\"any_is_na\"]\n"),
    ])?;

    // Both values set `ignore`; the last one decides, so `any_is_na` is reported.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("settings.toml")
            .arg("--config")
            .arg("lint.ignore = [\"any_duplicated\"]")
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
fn test_config_flag_inline_toml_syntax_error() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "any(is.na(x))")?;

    // A value that looks like TOML is reported as a snippet with a syntax error, not as a
    // path that does not exist.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("lint.ignore = ")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Failed to parse --config:
    TOML parse error at line 1, column 15
      |
    1 | lint.ignore = 
      |               ^
    "
    );

    Ok(())
}

#[test]
fn test_config_flag_snippet_only_anchors_at_the_working_directory() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "any(is.na(x))")?;

    // A snippet has no directory of its own, so its relative patterns are anchored where the command
    // runs: a leading `/` means the checked directory, not the filesystem root.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("lint.exclude = [\"/test.R\"]")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

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
fn test_config_flag_accepts_a_file_whose_name_contains_an_equals_sign() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("test.R", "any(is.na(x))\nany(duplicated(x))"),
        ("conf=prod.toml", "[lint]\nignore = [\"any_is_na\"]\n"),
    ])?;

    // A path is allowed to contain `=`: an existing file wins over the TOML reading of its name.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("conf=prod.toml")
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
