use crate::helpers::{CliTest, CommandExt};

#[test]
fn test_fix_and_add_jarl_ignore_incompatible() -> anyhow::Result<()> {
    let case = CliTest::with_files([("foo.R", "any(is.na(x))")])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--fix")
            .arg("--add-jarl-ignore")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: the argument '--fix' cannot be used with '--add-jarl-ignore[=<REASON>]'

    Usage: jarl check --fix <FILES>...

    For more information, try '--help'.
    "
    );

    Ok(())
}

#[test]
fn test_fix_only_and_add_jarl_ignore_incompatible() -> anyhow::Result<()> {
    let case = CliTest::with_files([("foo.R", "any(is.na(x))")])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--fix-only")
            .arg("--add-jarl-ignore")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: the argument '--fix-only' cannot be used with '--add-jarl-ignore[=<REASON>]'

    Usage: jarl check --fix-only <FILES>...

    For more information, try '--help'.
    "
    );

    Ok(())
}

#[test]
fn test_unsafe_fixes_and_add_jarl_ignore_incompatible() -> anyhow::Result<()> {
    let case = CliTest::with_files([("foo.R", "any(is.na(x))")])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--unsafe-fixes")
            .arg("--add-jarl-ignore")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: the argument '--unsafe-fixes' cannot be used with '--add-jarl-ignore[=<REASON>]'

    Usage: jarl check --unsafe-fixes <FILES>...

    For more information, try '--help'.
    "
    );

    Ok(())
}

#[test]
fn test_statistics_and_add_jarl_ignore_incompatible() -> anyhow::Result<()> {
    let case = CliTest::with_files([("foo.R", "any(is.na(x))")])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--statistics")
            .arg("--add-jarl-ignore")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: the argument '--statistics' cannot be used with '--add-jarl-ignore[=<REASON>]'

    Usage: jarl check --statistics <FILES>...

    For more information, try '--help'.
    "
    );

    Ok(())
}

#[test]
fn test_statistics_and_fix_incompatible() -> anyhow::Result<()> {
    let case = CliTest::with_files([("foo.R", "any(is.na(x))")])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--statistics")
            .arg("--fix")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: the argument '--statistics' cannot be used with '--fix'

    Usage: jarl check --statistics <FILES>...

    For more information, try '--help'.
    "
    );

    Ok(())
}

#[test]
fn test_statistics_and_fix_only_incompatible() -> anyhow::Result<()> {
    let case = CliTest::with_files([("foo.R", "any(is.na(x))")])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--statistics")
            .arg("--fix-only")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: the argument '--statistics' cannot be used with '--fix-only'

    Usage: jarl check --statistics <FILES>...

    For more information, try '--help'.
    "
    );

    Ok(())
}

#[test]
fn test_statistics_and_unsafe_fixes_incompatible() -> anyhow::Result<()> {
    let case = CliTest::with_files([("foo.R", "any(is.na(x))")])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--statistics")
            .arg("--unsafe-fixes")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: the argument '--statistics' cannot be used with '--unsafe-fixes'

    Usage: jarl check --statistics <FILES>...

    For more information, try '--help'.
    "
    );

    Ok(())
}
