use crate::helpers::{CliTest, CommandExt};

// ---------------------------------------------------------------------------
// Basic lint detection
// ---------------------------------------------------------------------------

#[test]
fn test_roxygen_examples_lint() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @param x A value
#' @examples
#' any(is.na(x))
foo <- function(x) x
",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/test.R:4:4
      |
    4 | #' any(is.na(x))
      |    ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_roxygen_examples_if_lint() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @examplesIf interactive()
#' any(is.na(x))
foo <- function(x) x
",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/test.R:3:4
      |
    3 | #' any(is.na(x))
      |    ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Clean examples produce no diagnostics
// ---------------------------------------------------------------------------

#[test]
fn test_roxygen_clean_examples() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @examples
#' x <- 1
foo <- function(x) x
",
        ),
    ])?;

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
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Parse errors silently skipped
// ---------------------------------------------------------------------------

#[test]
fn test_roxygen_parse_error_skipped() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @examples
#' 1 +
foo <- function(x) x
",
        ),
    ])?;

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
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Multiple roxygen blocks
// ---------------------------------------------------------------------------

#[test]
fn test_roxygen_multiple_blocks() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' First function
#' @examples
#' any(is.na(x))
foo <- function(x) x

#' Second function
#' @examples
#' any(is.na(y))
bar <- function(y) y
",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/test.R:3:4
      |
    3 | #' any(is.na(x))
      |    ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_is_na
     --> R/test.R:8:4
      |
    8 | #' any(is.na(y))
      |    ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.

    ----- stderr -----
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// check-roxygen = false disables roxygen linting
// ---------------------------------------------------------------------------

#[test]
fn test_roxygen_disabled_via_toml() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @examples
#' any(is.na(x))
foo <- function(x) x
",
        ),
        (
            "jarl.toml",
            "\
[lint]
check-roxygen = false
",
        ),
    ])?;

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
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Roxygen linting skipped for files outside an R package
// ---------------------------------------------------------------------------

#[test]
fn test_roxygen_skipped_outside_package() -> anyhow::Result<()> {
    // No DESCRIPTION, no R/ directory — just a plain R file
    let case = CliTest::with_file(
        "test.R",
        "\
#' Title
#' @examples
#' any(is.na(x))
foo <- function(x) x
",
    )?;

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
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// \dontrun{}, \donttest{}, \dontshow{} wrappers are stripped
// ---------------------------------------------------------------------------

/// Code inside `\dontrun{}` is linted — the wrapper is stripped.
#[test]
fn test_roxygen_dontrun_linted() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @examples
#' \\dontrun{
#' any(is.na(x))
#' }
foo <- function(x) x
",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/test.R:4:4
      |
    4 | #' any(is.na(x))
      |    ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

/// Code inside `\donttest{}` is linted — the wrapper is stripped.
#[test]
fn test_roxygen_donttest_linted() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @examples
#' \\donttest{
#' any(is.na(x))
#' }
foo <- function(x) x
",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/test.R:4:4
      |
    4 | #' any(is.na(x))
      |    ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

/// Code both inside and outside `\dontrun{}` is linted.
#[test]
fn test_roxygen_dontrun_with_surrounding_code() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @examples
#' any(is.na(x))
#' \\dontrun{
#' any(is.na(y))
#' }
foo <- function(x) x
",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/test.R:3:4
      |
    3 | #' any(is.na(x))
      |    ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.

    warning: any_is_na
     --> R/test.R:5:4
      |
    5 | #' any(is.na(y))
      |    ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.

    ----- stderr -----
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// @examples section stops at the next @tag
// ---------------------------------------------------------------------------

/// Code after `@return` (or any other tag) should NOT be linted as examples.
#[test]
fn test_roxygen_examples_stopped_by_tag() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' @title hi
#' @description
#' hello
#' @examples
#' any(is.na(x))
#' @return foo
#' any(is.na(x))
f <- function() 1
",
        ),
    ])?;

    // Only the first any(is.na(x)) (inside @examples) should be reported.
    // The second one is under @return and is not R code.
    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/test.R:5:4
      |
    5 | #' any(is.na(x))
      |    ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// fix-roxygen = true applies fixes at the correct position
// ---------------------------------------------------------------------------

/// Multi-line roxygen example is correctly fixed in place.
#[test]
fn test_roxygen_fix_multiline() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' @title hi
#' @description
#' hello
#' @examples
#' 1 + 1
#' any(
#'   is.na(x)
#' )
#' 1 + 1
#' @return foo
f <- function() 1
",
        ),
        (
            "jarl.toml",
            "\
[lint]
fix-roxygen = true
",
        ),
    ])?;

    case.command()
        .arg("check")
        .arg(".")
        .arg("--fix")
        .arg("--allow-no-vcs")
        .run();

    let fixed = case.read_file("R/test.R")?;
    insta::assert_snapshot!(
        fixed,
        @"
    #' @title hi
    #' @description
    #' hello
    #' @examples
    #' 1 + 1
    #' anyNA(x)
    #' 1 + 1
    #' @return foo
    f <- function() 1
    "
    );

    Ok(())
}

/// Single-line roxygen example is correctly fixed in place.
#[test]
fn test_roxygen_fix_single_line() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @examples
#' 1 + 1
#' any(is.na(x))
#' 1 + 1
foo <- function(x) x
",
        ),
        (
            "jarl.toml",
            "\
[lint]
fix-roxygen = true
",
        ),
    ])?;

    case.command()
        .arg("check")
        .arg(".")
        .arg("--fix")
        .arg("--allow-no-vcs")
        .run();

    let fixed = case.read_file("R/test.R")?;
    insta::assert_snapshot!(
        fixed,
        @"
    #' Title
    #' @examples
    #' 1 + 1
    #' anyNA(x)
    #' 1 + 1
    foo <- function(x) x
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// ##' is also a valid roxygen comment
// ---------------------------------------------------------------------------

#[test]
fn test_double_hash_is_roxygen() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
##' Title
##' @examples
##' any(is.na(x))
foo <- function(x) x
",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: any_is_na
     --> R/test.R:3:5
      |
    3 | ##' any(is.na(x))
      |     ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// suppression comments work
// ---------------------------------------------------------------------------

#[test]
fn test_suppression_comments() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @examples
# jarl-ignore any_is_na: <reason>
#' any(is.na(x))
foo <- function(x) x

#' Title
#' @examples
# jarl-ignore-start any_is_na: <reason>
#' any(is.na(x))
# jarl-ignore-end any_is_na
foo2 <- function(x) x
",
        ),
    ])?;

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
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_unused_suppression_comments() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @examples
# jarl-ignore any_duplicated: <reason>
#' any(is.na(x))
foo <- function(x) x
",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: outdated_suppression
     --> R/test.R:3:1
      |
    3 | # jarl-ignore any_duplicated: <reason>
      | -------------------------------------- This suppression comment is unused, no violation would be reported without it.
      |
      = help: Remove this suppression comment or verify that it's still needed.

    warning: any_is_na
     --> R/test.R:4:4
      |
    4 | #' any(is.na(x))
      |    ------------- `any(is.na(...))` is inefficient.
      |
      = help: Use `anyNA(...)` instead.


    ── Summary ──────────────────────────────────────
    Found 2 errors.
    1 fixable with the `--fix` option.

    ----- stderr -----
    "
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// unused_object in examples
// ---------------------------------------------------------------------------

/// Objects bound in an example live in the throwaway environment the example
/// runs in, so one that is never read is dead code just like in a script.
#[test]
fn test_roxygen_examples_unused_object() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @examples
#' d <- data.frame(a = 1)
#' summary(mtcars)
foo <- function() NULL
",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_object
     --> R/test.R:3:4
      |
    3 | #' d <- data.frame(a = 1)
      |    - Object `d` is defined but never used.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

/// A binding read later in the same section is used.
#[test]
fn test_roxygen_examples_used_object_not_flagged() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @examples
#' d <- data.frame(a = 1)
#' summary(d)
foo <- function() NULL
",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

/// `@examplesIf` sections and `\dontrun{}` bodies are analysed like any other
/// example code.
#[test]
fn test_roxygen_examples_if_and_dontrun_unused_object() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @examplesIf interactive()
#' b <- 1
f2 <- function() NULL

#' Title
#' @examples
#' \\dontrun{
#' a <- 1
#' }
f1 <- function() NULL
",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_object
     --> R/test.R:3:4
      |
    3 | #' b <- 1
      |    - Object `b` is defined but never used.
      |

    warning: unused_object
     --> R/test.R:9:4
      |
    9 | #' a <- 1
      |    - Object `a` is defined but never used.
      |


    ── Summary ──────────────────────────────────────
    Found 2 errors.

    ----- stderr -----
    "
    );

    Ok(())
}

/// String interpolation counts as a read, which requires the documented file's
/// package context to reach the roxygen analysis: `glue` from DESCRIPTION for
/// the first block, `library(glue)` in the example itself for the second.
#[test]
fn test_roxygen_examples_interpolation_is_a_read() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\nImports: glue\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @examples
#' nm <- \"world\"
#' glue(\"hello {nm}\")
foo <- function() NULL

#' Title
#' @examples
#' library(glue)
#' other <- \"world\"
#' glue(\"hello {other}\")
bar <- function() NULL
",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

/// An example binds into a throwaway environment, so it is never the package
/// object of the same name: exporting `bar` does not make the example's `bar`
/// used, and a sibling file reading `bar` does not either.
#[test]
fn test_roxygen_examples_exported_name_still_flagged() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        ("NAMESPACE", "export(bar)\n"),
        (
            "R/test.R",
            "\
#' Title
#' @examples
#' bar <- 1
baz <- function() NULL
",
        ),
        ("R/other.R", "bar <- 1\nuse_it <- function() bar\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_object
     --> R/test.R:3:4
      |
    3 | #' bar <- 1
      |    --- Object `bar` is defined but never used.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

/// Each roxygen block is its own example, so a binding in one is not read by
/// the next.
#[test]
fn test_roxygen_examples_blocks_are_independent() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @examples
#' shared <- 1
foo <- function() NULL

#' Title
#' @examples
#' print(shared)
bar <- function() NULL
",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_object
     --> R/test.R:3:4
      |
    3 | #' shared <- 1
      |    ------ Object `shared` is defined but never used.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

/// A `# jarl-ignore` comment in the documented file suppresses a violation
/// found inside the examples.
#[test]
fn test_roxygen_examples_unused_object_suppressed() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @examples
# jarl-ignore unused_object: illustrative
#' zz <- 1
qux <- function() NULL
",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

/// An examples section with no code at all must not trip document-level rules.
#[test]
fn test_roxygen_examples_comment_only_section() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "R/test.R",
            "\
#' Title
#' @examples
#' # see the vignette
quux <- function() NULL
",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object,empty_file")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}
