use crate::helpers::{CliTest, CommandExt};
use crate::stdin::check_stdin;

const CODE: &str = "any(is.na(x))\nany(duplicated(x))\nx$a\n";

#[test]
fn test_config_override_without_jarl_toml() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", CODE)?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("lint.select = ['any_is_na']")
            .run()
            .normalize_os_executable_name(),
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
fn test_config_override_keeps_other_options_of_jarl_toml() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("test.R", CODE),
        (
            "jarl.toml",
            "[lint]\nselect = [\"undesirable_operator\", \"any_is_na\"]\n\n[lint.undesirable_operator]\noperators = [\"<<-\"]\n",
        ),
    ])?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("lint.undesirable_operator.operators = ['$']")
            .run()
            .normalize_os_executable_name(),
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

    warning: undesirable_operator
     --> test.R:3:2
      |
    3 | x$a
      |  - `$` is listed as an undesirable operator.
      |


    ── Summary ──────────────────────────────────────
    Found 2 errors.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );
    Ok(())
}

#[test]
fn test_config_override_applies_to_nested_jarl_toml() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("jarl.toml", "[lint]\nselect = [\"any_is_na\"]\n"),
        ("test.R", CODE),
        ("sub/jarl.toml", "[lint]\nselect = [\"any_is_na\"]\n"),
        ("sub/test.R", CODE),
    ])?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("lint.select = ['any_duplicated']")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_duplicated
     --> sub/test.R:2:1
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
fn test_several_config_overrides() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", CODE)?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("lint.select = ['any_is_na']")
            .arg("--config")
            .arg("lint.extend-select = ['any_duplicated']")
            // The last value of an option wins.
            .arg("--config")
            .arg("lint.select = ['undesirable_operator']")
            .arg("--config")
            .arg("lint.undesirable_operator.operators = ['$']")
            .run()
            .normalize_os_executable_name(),
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

    warning: undesirable_operator
     --> test.R:3:2
      |
    3 | x$a
      |  - `$` is listed as an undesirable operator.
      |


    ── Summary ──────────────────────────────────────
    Found 2 errors.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );
    Ok(())
}

#[test]
fn test_config_file_replaces_discovered_jarl_toml() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("jarl.toml", "[lint]\nselect = [\"any_is_na\"]\n"),
        ("test.R", CODE),
        ("sub/jarl.toml", "[lint]\nselect = [\"any_is_na\"]\n"),
        ("sub/test.R", CODE),
        (
            "configs/custom.toml",
            "[lint]\nselect = [\"any_duplicated\"]\n",
        ),
    ])?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("configs/custom.toml")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_duplicated
     --> sub/test.R:2:1
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
fn test_config_file_and_overrides() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("test.R", CODE),
        ("custom.toml", "[lint]\nselect = [\"any_duplicated\"]\n"),
    ])?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("lint.extend-select = ['any_is_na']")
            .arg("--config")
            .arg("custom.toml")
            .run()
            .normalize_os_executable_name(),
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
fn test_config_file_relative_paths_resolved_from_cwd() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("R/excluded.R", CODE),
        ("R/kept.R", CODE),
        (
            "configs/custom.toml",
            "[lint]\nselect = [\"any_is_na\"]\nexclude = [\"R/excluded.R\"]\n",
        ),
    ])?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("configs/custom.toml")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/kept.R:1:1
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
fn test_config_file_applies_to_files_outside_cwd() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("other/test.R", CODE),
        ("work/custom.toml", "[lint]\nselect = [\"any_is_na\"]\n"),
    ])?;
    // `--statistics` avoids printing the path of a file outside of the cwd,
    // which depends on the platform.
    insta::assert_snapshot!(
        &mut case
            .command()
            .current_dir(case.root().join("work"))
            .arg("check")
            .arg("../other")
            .arg("--statistics")
            .arg("--config")
            .arg("custom.toml")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
        1 [*] any_is_na

    Rules with `[*]` have an automatic safe fix.
    Rules with `[^]` have an automatic unsafe fix.

    ----- stderr -----
    "
    );
    Ok(())
}

#[test]
fn test_config_with_stdin() -> anyhow::Result<()> {
    let case = CliTest::with_file("custom.toml", "[lint]\nselect = [\"any_is_na\"]\n")?;
    insta::assert_snapshot!(
        check_stdin(
            &case,
            CODE,
            &[
                "--stdin-file-path",
                "foo.R",
                "--output-format",
                "concise",
                "--config",
                "custom.toml",
            ],
        )?,
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    foo.R [1:1] any_is_na `any(is.na(...))` is inefficient. Use `anyNA(...)` instead.

    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );
    Ok(())
}

fn case_with_config_in(dir: &str) -> anyhow::Result<CliTest> {
    CliTest::with_files([
        ("test.R", CODE),
        (
            &format!("{dir}/custom.toml"),
            "[lint]\nselect = [\"any_duplicated\"]\n",
        ),
    ])
}

#[test]
fn test_config_file_absolute_path() -> anyhow::Result<()> {
    let case = case_with_config_in("configs")?;
    let path = case.root().join("configs").join("custom.toml");
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg(path)
            .run()
            .normalize_os_executable_name(),
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
fn test_config_file_dot_relative_path() -> anyhow::Result<()> {
    let case = case_with_config_in("configs")?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("./configs/custom.toml")
            .run()
            .normalize_os_executable_name(),
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
fn test_config_file_path_with_spaces() -> anyhow::Result<()> {
    let case = case_with_config_in("my configs")?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("my configs/custom.toml")
            .run()
            .normalize_os_executable_name(),
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
fn test_config_file_parent_relative_path() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("project/test.R", CODE),
        ("custom.toml", "[lint]\nselect = [\"any_duplicated\"]\n"),
    ])?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .current_dir(case.root().join("project"))
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("../custom.toml")
            .run()
            .normalize_os_executable_name(),
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
fn test_config_file_without_toml_extension() -> anyhow::Result<()> {
    let case = case_with_config_in("configs")?;
    std::fs::rename(
        case.root().join("configs/custom.toml"),
        case.root().join("configs/custom"),
    )?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("configs/custom")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: invalid value 'configs/custom' for '--config <CONFIG_OPTION>': configuration file must be a TOML file

    For more information, try '--help'.
    "
    );
    Ok(())
}

#[test]
fn test_config_only_one_file() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        ("test.R", CODE),
        ("a.toml", "[lint]\n"),
        ("b.toml", "[lint]\n"),
    ])?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("a.toml")
            .arg("--config")
            .arg("b.toml")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: `--config` can be a path to a configuration file only once.
    "
    );
    Ok(())
}

#[test]
fn test_config_missing_file() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", CODE)?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("configs/nope.toml")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: invalid value 'configs/nope.toml' for '--config <CONFIG_OPTION>': configuration file 'configs/nope.toml' does not exist

    For more information, try '--help'.
    "
    );
    Ok(())
}

#[test]
fn test_config_invalid_toml() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", CODE)?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("lint.select = any_is_na")
            .run()
            .normalize_os_executable_name(),
        @r#"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: invalid value 'lint.select = any_is_na' for '--config <CONFIG_OPTION>': neither a path to an existing file nor a valid TOML `<KEY> = <VALUE>` pair, e.g. `"lint.select = ['any_is_na']"`:
    TOML parse error at line 1, column 15
      |
    1 | lint.select = any_is_na
      |               ^^^^^^^^^
    string values must be quoted, expected literal string


    For more information, try '--help'.
    "#
    );
    Ok(())
}

#[test]
fn test_config_unknown_option() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", CODE)?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("lint.foo = 1")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: invalid value 'lint.foo = 1' for '--config <CONFIG_OPTION>': Invalid configuration in --config:
    Unknown field `foo` in `[lint]`. Expected one of: `select`, `extend-select`, `ignore`, `fixable`, `unfixable`, `exclude`, `default-exclude`, `include`, `per-file-ignores`, `check-roxygen`, `fix-roxygen`, `ignore-roxygen`.

    For more information, try '--help'.
    "
    );
    Ok(())
}

#[test]
fn test_config_invalid_file() -> anyhow::Result<()> {
    let case = CliTest::with_files([("test.R", CODE), ("custom.toml", "[lint]\nfoo = 1\n")])?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("custom.toml")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Invalid configuration in [TEMP_DIR]/custom.toml:
    Unknown field `foo` in `[lint]`. Expected one of: `select`, `extend-select`, `ignore`, `fixable`, `unfixable`, `exclude`, `default-exclude`, `include`, `per-file-ignores`, `check-roxygen`, `fix-roxygen`, `ignore-roxygen`.
    "
    );
    Ok(())
}

#[test]
fn test_config_override_adds_table_before_other_headers() -> anyhow::Result<()> {
    // `[lint]` is absent from `jarl.toml`: written after `[lint.assignment]`,
    // `select` would be read as an option of `assignment`.
    let case = CliTest::with_files([
        ("test.R", "x = 1\nany(is.na(x))\n"),
        ("jarl.toml", "[lint.assignment]\noperator = \"<-\"\n"),
    ])?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("lint.select = ['assignment']")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: assignment
     --> test.R:1:1
      |
    1 | x = 1
      | --- Use `<-` for assignment.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );
    Ok(())
}

#[test]
fn test_config_override_with_quoted_inline_table_keys() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", CODE)?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("lint.select = ['undesirable_operator']")
            .arg("--config")
            .arg(r#"lint.undesirable_operator.operators = [{ "$" = "Use [[ instead." }]"#)
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: undesirable_operator
     --> test.R:3:2
      |
    3 | x$a
      |  - `$` is listed as an undesirable operator.
      |
      = help: Use [[ instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );
    Ok(())
}

#[test]
fn test_config_override_with_unquoted_inline_table_keys() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", CODE)?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg(r#"lint.undesirable_operator.operators = [{ foo = "Don't." }]"#)
            .run()
            .normalize_os_executable_name(),
        @r#"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: invalid value 'lint.undesirable_operator.operators = [{ foo = "Don't." }]' for '--config <CONFIG_OPTION>': Invalid configuration in --config:
    Key `foo` in `operators` of `[lint.undesirable_operator]` must be quoted. Use `"foo"` instead.

    For more information, try '--help'.
    "#
    );
    Ok(())
}

#[test]
fn test_config_override_with_malformed_jarl_toml() -> anyhow::Result<()> {
    // The syntax error is reported at its position in `jarl.toml`, not in a
    // document merged with the overrides.
    let case = CliTest::with_files([("test.R", CODE), ("jarl.toml", "[lint]\nselect = [\n")])?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--config")
            .arg("lint.select = ['any_is_na']")
            .run()
            .normalize_os_executable_name()
            .normalize_temp_paths(),
        @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    jarl failed
      Cause: Failed to parse [TEMP_DIR]/jarl.toml:
    TOML parse error at line 2, column 11
      |
    2 | select = [
      |           ^
    unclosed array, expected `]`
    "
    );
    Ok(())
}
