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

/// Check rule that is disabled by default
#[test]
fn test_known_rule_disabled_by_default() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    insta::assert_snapshot!(
        case.command()
            .arg("rule")
            .arg("assignment")
            .run()
            .normalize_os_executable_name(),
        @r#"

    success: true
    exit_code: 0
    ----- stdout -----
    assignment
    Categories: READ
    Enabled by default: no
    Fix: safe

    Added in 0.0.8

    ## What it does

    Checks for consistency of assignment operator.

    ## Why is this bad?

    In most cases using `=` and `<-` is equivalent. Some very popular packages
    use `=` without problems. This rule only ensures the consistency of the
    assignment operator in a project.

    Set the following option in `jarl.toml` to use `=` as the preferred operator:

    ```toml
    [lint.assignment]
    operator = "=" # or "<-"
    ```

    ## Example

    If the `operator` parameter is `"="` then replace:
    ```r
    x <- "a"
    ```
    by:
    ```r
    x = "a"
    ```

    Note that Jarl will not report some cases where `<-` is used because it
    would change the meaning of code, e.g. this:

    ```r
    f(x <- 1)
    ```
    cannot be replaced by:

    ```r
    f(x = 1)
    ```

    ## References

    See:

    - [https://style.tidyverse.org/syntax.html#assignment-1](https://style.tidyverse.org/syntax.html#assignment-1)

    ----- stderr -----
    "#
    );

    Ok(())
}

/// Check rule that has no fix
#[test]
fn test_known_rule_with_no_fix() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    insta::assert_snapshot!(
        case.command()
            .arg("rule")
            .arg("if_always_true")
            .run()
            .normalize_os_executable_name(),
        @r#"

    success: true
    exit_code: 0
    ----- stdout -----
    if_always_true
    Categories: READ, SUSP
    Enabled by default: yes
    Fix: not available

    Added in 0.4.0

    ## What it does

    Detects `if` conditions that always evaluate to `TRUE`. This is only triggered
    for `if` statements without an `else` clause, these are handled by
    `unreachable_code`.

    ## Why is this bad?

    Code in an `if` statement whose condition always evaluates to `TRUE` will
    always run. It clutters the code and makes it more difficult to read. In
    these cases, the `if` condition should be removed.

    This rule does not have an automatic fix.

    ## Example

    ```r
    if (TRUE) {
      print("always true")
    }

    if (TRUE || ...) {
      print("always true")
    }

    if (!FALSE) {
      print("always true")
    }
    ```

    Use instead:

    ```r
    print("always true")
    ```

    ----- stderr -----
    "#
    );

    Ok(())
}

/// Check rule that has a minimum R version
#[test]
fn test_known_rule_with_min_r_version() -> anyhow::Result<()> {
    let case = CliTest::new()?;
    insta::assert_snapshot!(
        case.command()
            .arg("rule")
            .arg("grepv")
            .run()
            .normalize_os_executable_name(),
        @r#"

    success: true
    exit_code: 0
    ----- stdout -----
    grepv
    Categories: READ
    Enabled by default: yes
    Fix: safe
    Minimum R version: 4.5.0

    Added in 0.0.16

    ## What it does

    Checks for usage of `grep(..., value = TRUE)` and recommends using
    `grepv()` instead (only if the R version used in the project is >= 4.5).

    ## Why is this bad?

    Starting from R 4.5, there is a function `grepv()` that is identical to
    `grep()` except that it uses `value = TRUE` by default.

    Using `grepv(...)` is therefore more readable than `grep(...)`.

    ## Example

    ```r
    x <- c("hello", "hi", "howdie")
    grep("i", x, value = TRUE)
    ```

    Use instead:
    ```r
    x <- c("hello", "hi", "howdie")
    grepv("i", x)
    ```

    ## References

    See `?grepv`

    ----- stderr -----
    "#
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
    Print the documentation of a rule

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
