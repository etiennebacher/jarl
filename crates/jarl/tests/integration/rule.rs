use crate::helpers::{CliTest, CommandExt};

/// A known rule prints its metadata header followed by the embedded
/// documentation.
#[test]
fn test_known_rule_prints_docs() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    insta::assert_snapshot!(
        case.command()
            .arg("rule")
            .arg("all_equal")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    all_equal
    Categories: SUSP
    Enabled by default: yes
    Fix: unsafe (requires `--unsafe-fixes`)

    Added in 0.0.19

    ## What it does

    Checks for dangerous usage of `all.equal(...)`, for instance in `if()`
    conditions or `while()` loops.

    ## Why is this bad?

    `all.equal()` returns `TRUE` in the absence of differences but returns a
    character string (not `FALSE`) in the presence of differences. Usage of
    `all.equal()` without wrapping it in `isTRUE()` are thus likely to generate
    unexpected errors if the compared objects have differences. An alternative
    is to use `identical()` to compare vector of strings or when exact equality
    is expected.

    This rule has automated fixes that are marked unsafe and therefore require
    passing `--unsafe-fixes`. This is because automatically fixing those cases
    can change the runtime behavior if some code relied on the behaviour of
    `all.equal()` (likely by mistake).

    ## Example

    ```r
    a <- 1
    b <- 1

    if (all.equal(a, b, tolerance = 1e-3)) message('equal')
    if (all.equal(a, b)) message('equal')
    !all.equal(a, b)
    isFALSE(all.equal(a, b))

    ```

    Use instead:
    ```r
    a <- 1
    b <- 1

    if (isTRUE(all.equal(a, b, tolerance = 1e-3))) message('equal')
    if (isTRUE(all.equal(a, b))) message('equal')
    !isTRUE(all.equal(a, b))
    !isTRUE(all.equal(a, b))
    ```

    ## References

    See `?all.equal`

    ----- stderr -----
    "
    );

    Ok(())
}

/// A deprecated rule reports its deprecation in the metadata header.
#[test]
fn test_deprecated_rule_shows_note() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    insta::assert_snapshot!(
        case.command()
            .arg("rule")
            .arg("browser")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    browser
    Categories: CORR
    Enabled by default: yes
    Fix: safe
    Note: deprecated since 0.5.0, use `undesirable_function` instead

    Added in 0.1.2

    ## What it does

    Checks for lingering presence of `browser()` which should not be present in
    released code.

    **This rule is deprecated and will be removed in a future version. Use the
    rule [`undesirable_function`](https://jarl.etiennebacher.com/rules/undesirable_function)
    and configure it to report calls to `browser()` instead.**

    ## Why is this bad?

    `browser()` interrupts the execution of an expression and allows the inspection
    of the environment where `browser()` was called from. This is helpful while
    developing a function, but is not expected to be called by the user. Does not
    remove the call as it does not have a suitable replacement.

    ## Example

    ```r
    do_something <- function(abc = 1) {
       xyz <- abc + 1
       browser()      # This should be removed.
       xyz
    }

    ```

    ## References

    See `?browser`

    ----- stderr -----
    "
    );

    Ok(())
}

/// An unknown rule fails with a helpful error and a "did you mean" suggestion.
#[test]
fn test_unknown_rule_errors_with_suggestion() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    insta::assert_snapshot!(
        case.command()
            .arg("rule")
            .arg("all_equl")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: unknown rule `all_equl`.
      Did you mean `all_equal`?
    Run `jarl check --help` for how to select rules.
    "
    );

    Ok(())
}

/// Calling `jarl rule` without a rule name shows the subcommand help.
#[test]
fn test_rule_without_name_shows_help() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    insta::assert_snapshot!(
        case.command()
            .arg("rule")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Explain a rule (print its documentation)

    Usage: jarl rule [OPTIONS] <NAME>

    Arguments:
      <NAME>  Name of the rule to explain, for example `jarl rule all_equal`.

    Options:
      -h, --help  Print help

    Global options:
          --log-level <LOG_LEVEL>  The log level. One of: `error`, `warn`, `info`, `debug`, or `trace`. Defaults to `warn`
    "
    );

    Ok(())
}
