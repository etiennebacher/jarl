use crate::helpers::{CliTest, CommandExt, Output};

/// Lint a single file containing `source` and return the CLI output.
///
/// Syntax errors are reported on stderr, so the interesting part of the
/// snapshot is always the `----- stderr -----` section.
fn check(source: &str) -> anyhow::Result<Output> {
    let case = CliTest::with_file("test.R", source)?;
    Ok(case.command().arg("check").arg(".").run())
}

#[test]
fn test_expected_an_expression() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("repeat")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected an expression
     --> test.R:1:7
      |
    1 | repeat
      |       ^
      |
    ");
    Ok(())
}

#[test]
fn test_expected_an_identifier_in_for_loop_variable() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("for (1 in x) 1")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected an identifier
     --> test.R:1:6
      |
    1 | for (1 in x) 1
      |      ^
      |
    ");
    Ok(())
}

#[test]
fn test_expected_an_identifier_in_parameter() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("function(1) 1")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected an identifier
     --> test.R:1:10
      |
    1 | function(1) 1
      |          ^
      |
    ");
    Ok(())
}

#[test]
fn test_expected_double_bracket() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("x[[1] ]")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected `]]`
     --> test.R:1:5
      |
    1 | x[[1] ]
      |     ^
      |
    ");
    Ok(())
}

#[test]
fn test_expected_a_comma_between_arguments() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("f(a b)")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected a comma between arguments
     --> test.R:1:5
      |
    1 | f(a b)
      |     ^
      |
    ");
    Ok(())
}

#[test]
fn test_expected_an_identifier_or_string_before_double_colon() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("f()::x")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected an identifier or string before `::`
     --> test.R:1:4
      |
    1 | f()::x
      |    ^^
      |
    ");
    Ok(())
}

#[test]
fn test_expected_an_identifier_or_string_before_triple_colon() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("f():::x")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected an identifier or string before `:::`
     --> test.R:1:4
      |
    1 | f():::x
      |    ^^^
      |
    ");
    Ok(())
}

#[test]
fn test_expected_a_parameter() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("function(x, ) 1")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected a parameter
     --> test.R:1:13
      |
    1 | function(x, ) 1
      |             ^
      |
    ");
    Ok(())
}

#[test]
fn test_expected_in() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("for (x y) 1")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected `in`
     --> test.R:1:8
      |
    1 | for (x y) 1
      |        ^
      |
    ");
    Ok(())
}

#[test]
fn test_expected_closing_paren_at_eof() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("any(")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected `)` but instead the file ends
     --> test.R:1:5
      |
    1 | any(
      |     ^
      |
    ");
    Ok(())
}

#[test]
fn test_expected_closing_paren_found_other_token() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("f <- function(x { }")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected `)` but instead found `{`
     --> test.R:1:17
      |
    1 | f <- function(x { }
      |                 ^
      |
    ");
    Ok(())
}

#[test]
fn test_expected_opening_paren_at_eof() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("while")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected `(` but instead the file ends
     --> test.R:1:6
      |
    1 | while
      |      ^
      |
    ");
    Ok(())
}

#[test]
fn test_expected_opening_paren_found_other_token() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("if x")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected `(` but instead found `x`
     --> test.R:1:4
      |
    1 | if x
      |    ^
      |
    ");
    Ok(())
}

#[test]
fn test_expected_closing_brace_at_eof() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("{ x")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected `}` but instead the file ends
     --> test.R:1:4
      |
    1 | { x
      |    ^
      |
    ");
    Ok(())
}

#[test]
fn test_unterminated_string() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("x <- \"abc")?, @r#"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected the end of the string
     --> test.R:1:7
      |
    1 | x <- "abc
      |       ^^^
      |
    error: Unterminated string.
     --> test.R:1:7
      |
    1 | x <- "abc
      |       ^^^
      |
    "#);
    Ok(())
}

/// The opening quote swallows the rest of the file, so the span covers several
/// lines and the snippet is rendered as a multi-line annotation.
#[test]
fn test_unterminated_string_before_valid_code() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("x <- \"abc\ny <- 1")?, @r#"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected the end of the string
     --> test.R:1:7
      |
    1 |   x <- "abc
      |  _______^
    2 | | y <- 1
      | |______^
      |
    error: Unterminated string.
     --> test.R:1:7
      |
    1 |   x <- "abc
      |  _______^
    2 | | y <- 1
      | |______^
      |
    "#);
    Ok(())
}

#[test]
fn test_unterminated_raw_string() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("x <- r\"(abc")?, @r#"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected the end of the string
     --> test.R:1:9
      |
    1 | x <- r"(abc
      |         ^^^
      |
    error: Unterminated raw string.
     --> test.R:1:9
      |
    1 | x <- r"(abc
      |         ^^^
      |
    "#);
    Ok(())
}

#[test]
fn test_unterminated_quoted_identifier() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("`abc")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected an expression
     --> test.R:1:1
      |
    1 | `abc
      | ^^^^
      |
    error: Unterminated quoted identifier.
     --> test.R:1:1
      |
    1 | `abc
      | ^^^^
      |
    ");
    Ok(())
}

#[test]
fn test_unterminated_special_operator() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("x %in")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected an expression
     --> test.R:1:3
      |
    1 | x %in
      |   ^^^
      |
    error: Unterminated special operator.
     --> test.R:1:3
      |
    1 | x %in
      |   ^^^
      |
    ");
    Ok(())
}

/// Semicolons are only legal as statement separators; anywhere else the token
/// source records them.
#[test]
fn test_unexpected_semicolon() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("f(a;)")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: Unexpected `;`.
     --> test.R:1:4
      |
    1 | f(a;)
      |    ^
      |
    ");
    Ok(())
}

/// Independent errors are reported one by one instead of collapsing into a
/// single "this file failed to parse" line.
#[test]
fn test_several_syntax_errors_in_one_file() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("f(a b)\nfor (x y) 1\nz[[1] ]")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected a comma between arguments
     --> test.R:1:5
      |
    1 | f(a b)
      |     ^
      |
    error: expected `in`
     --> test.R:2:8
      |
    2 | for (x y) 1
      |        ^
      |
    error: expected `]]`
     --> test.R:3:5
      |
    3 | z[[1] ]
      |     ^
      |
    ");
    Ok(())
}

/// An error at the very end of the file has a zero-width span past the final
/// newline; it is moved back to the end of the last line with content so the
/// snippet isn't empty.
#[test]
fn test_syntax_error_at_end_of_file_with_trailing_newline() -> anyhow::Result<()> {
    insta::assert_snapshot!(check("any(\n")?, @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    error: expected `)` but instead the file ends
     --> test.R:1:5
      |
    1 | any(
      |     ^
      |
    ");
    Ok(())
}

/// The concise emitter reports the same errors as `file:row:col message`.
#[test]
fn test_several_syntax_errors_concise() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "f(a b)\nfor (x y) 1\nz[[1] ]")?;
    insta::assert_snapshot!(
        case.command()
            .arg("check")
            .arg(".")
            .arg("--output-format")
            .arg("concise")
            .run(),
        @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    Error: test.R:1:5 expected a comma between arguments
    Error: test.R:2:8 expected `in`
    Error: test.R:3:5 expected `]]`
    "
    );
    Ok(())
}

/// Rmd chunks that fail to parse are dropped before linting, so the only way
/// the combined source still fails is a chunk that is valid on its own but not
/// once concatenated — such as a BOM opening a chunk that is not the first.
/// Ranges in the combined source can't be mapped back to the Rmd file, so this
/// is the one case reported as the generic summary line rather than per-error
/// snippets.
#[test]
fn test_rmd_parse_error_reports_generic_summary() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "```{r}\nx <- 1\n```\n\n```{r}\n\u{feff}y <- 2\n```\n",
    )?;
    insta::assert_snapshot!(case.command().arg("check").arg(".").run(), @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    Error: Failed to parse test.Rmd due to syntax errors.
    ");
    Ok(())
}

/// The same Rmd fallback in the concise emitter.
#[test]
fn test_rmd_parse_error_reports_generic_summary_concise() -> anyhow::Result<()> {
    let case = CliTest::with_file(
        "test.Rmd",
        "```{r}\nx <- 1\n```\n\n```{r}\n\u{feff}y <- 2\n```\n",
    )?;
    insta::assert_snapshot!(
        case.command()
            .arg("check")
            .arg(".")
            .arg("--output-format")
            .arg("concise")
            .run(),
        @"

    success: false
    exit_code: 255
    ----- stdout -----

    ----- stderr -----
    Error: Failed to parse test.Rmd due to syntax errors.
    "
    );
    Ok(())
}
