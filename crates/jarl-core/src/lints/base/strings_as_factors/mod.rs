pub(crate) mod strings_as_factors;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "strings_as_factors", Some("3.6"))
    }

    #[test]
    fn test_no_lint_strings_as_factors() {
        // The rule only applies when the project explicitly supports R < 4.0.
        expect_no_lint("data.frame(x = \"a\")", "strings_as_factors", Some("4.0"));
        expect_no_lint("data.frame(x = \"a\")", "strings_as_factors", Some("4.5"));
        expect_no_lint("data.frame(x = \"a\")", "strings_as_factors", None);

        expect_no_lint(
            "data.frame(x = \"a\", stringsAsFactors = TRUE)",
            "strings_as_factors",
            Some("3.6"),
        );
        expect_no_lint(
            "data.frame(x = \"a\", stringsAsFactors = FALSE)",
            "strings_as_factors",
            Some("3.6"),
        );
        expect_no_lint(
            "data.frame(x = \"a\", stringsAsFactors = option)",
            "strings_as_factors",
            Some("3.6"),
        );

        expect_no_lint("data.frame(x = 1.2)", "strings_as_factors", Some("3.6"));
        expect_no_lint(
            "data.frame(row.names = \"a\")",
            "strings_as_factors",
            Some("3.6"),
        );
        expect_no_lint(
            "data.frame(a = c('b c' = 1))",
            "strings_as_factors",
            Some("3.6"),
        );
        expect_no_lint(
            "data.frame(x = c(\"a\", y))",
            "strings_as_factors",
            Some("3.6"),
        );
        expect_no_lint(
            "data.frame(x = c(\"a\", 1 + 2))",
            "strings_as_factors",
            Some("3.6"),
        );
        expect_no_lint(
            "data.frame(x = rep(c(\"a\", y), 2))",
            "strings_as_factors",
            Some("3.6"),
        );
        expect_no_lint(
            "data.frame(x = rep(times = 2, x = \"a\"))",
            "strings_as_factors",
            Some("3.6"),
        );
        expect_no_lint(
            "data.frame(row.names = as.character(y))",
            "strings_as_factors",
            Some("3.6"),
        );
        expect_no_lint(
            "data.frame(x = I(\"a\"))",
            "strings_as_factors",
            Some("3.6"),
        );
        expect_no_lint("tibble(x = \"a\")", "strings_as_factors", Some("3.6"));
        expect_no_lint("data.frame(\"a b\" = 1)", "strings_as_factors", Some("3.6"));
    }

    #[test]
    fn test_lint_strings_as_factors() {
        assert_snapshot!(
            snapshot_lint("data.frame(x = \"a\")"),
            @r#"
        warning: strings_as_factors
         --> <test>:1:1
          |
        1 | data.frame(x = "a")
          | ------------------- `data.frame()` can create different column types before and after R 4.0 when `stringsAsFactors` is omitted.
          |
          = help: Specify `stringsAsFactors = TRUE` or `stringsAsFactors = FALSE` explicitly.
        Found 1 error.
        "#
        );

        assert_snapshot!(
            snapshot_lint("data.frame('a')"),
            @r#"
        warning: strings_as_factors
         --> <test>:1:1
          |
        1 | data.frame('a')
          | --------------- `data.frame()` can create different column types before and after R 4.0 when `stringsAsFactors` is omitted.
          |
          = help: Specify `stringsAsFactors = TRUE` or `stringsAsFactors = FALSE` explicitly.
        Found 1 error.
        "#
        );

        assert_snapshot!(
            snapshot_lint("data.frame(x = \"a\", row.names = \"row\")"),
            @r#"
        warning: strings_as_factors
         --> <test>:1:1
          |
        1 | data.frame(x = "a", row.names = "row")
          | -------------------------------------- `data.frame()` can create different column types before and after R 4.0 when `stringsAsFactors` is omitted.
          |
          = help: Specify `stringsAsFactors = TRUE` or `stringsAsFactors = FALSE` explicitly.
        Found 1 error.
        "#
        );
    }

    #[test]
    fn test_lint_static_character_expressions() {
        let cases = [
            "data.frame(x = c(\"a\", \"b\"))",
            "data.frame(x = c(\"a\", 1))",
            "data.frame(x = c(\"a\", TRUE))",
            "data.frame(x = c(\"a\", FALSE))",
            "data.frame(x = c(\"a\", NA))",
            "data.frame(x = c(\"a\", NULL))",
            "data.frame(x = c(\"a\", Inf))",
            "data.frame(x = c(\"a\", NaN))",
            "data.frame(x = c(\"a\", 1i))",
            "data.frame(x = c(, \"a\"))",
            "data.frame(x = rep(\"a\", 2))",
            "data.frame(x = rep(c(\"a\", \"b\"), 2))",
            "data.frame(x = rep(x = \"a\", times = 2))",
        ];

        for code in cases {
            assert_eq!(
                check_code(code, "strings_as_factors", Some("3.6")).len(),
                1,
                "expected a lint for `{code}`"
            );
        }

        for function in [
            "character",
            "as.character",
            "paste",
            "sprintf",
            "format",
            "formatC",
            "prettyNum",
            "toString",
            "encodeString",
        ] {
            let code = format!("data.frame(x = {function}(y))");
            assert_eq!(
                check_code(&code, "strings_as_factors", Some("3.6")).len(),
                1,
                "expected a lint for `{code}`"
            );
        }
    }
}
