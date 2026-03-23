use crate::helpers::{CliTest, CommandExt};

#[test]
fn test_no_default_exclude() -> anyhow::Result<()> {
    let case = CliTest::with_file("cpp11.R", "any(is.na(x))")?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    Warning: No R files found under the given path(s).

    ----- stderr -----
    "
    );

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--no-default-exclude")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> cpp11.R:1:1
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
fn test_no_default_exclude_overrides_toml() -> anyhow::Result<()> {
    let case = CliTest::with_file("cpp11.R", "any(is.na(x))")?;
    case.write_file(
        "jarl.toml",
        r#"
[lint]
default-exclude = true
"#,
    )?;
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--no-default-exclude")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> cpp11.R:1:1
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
