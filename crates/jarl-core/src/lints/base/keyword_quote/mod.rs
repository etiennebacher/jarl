pub(crate) mod keyword_quote;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "keyword_quote", None)
    }

    #[test]
    fn test_no_lint_keyword_quote_call() {
        expect_no_lint("c(a = 1, 2)", "keyword_quote", None);
        // Non-syntactic names can use quotes or backticks
        expect_no_lint(
            "c(`a b` = 1, \"c d\" = 2, 'e f' = 3)",
            "keyword_quote",
            None,
        );
        // Strings as values or unnamed arguments
        expect_no_lint("c(A = \"a\", \"b\")", "keyword_quote", None);
        // Missing argument
        expect_no_lint("alist(`a b` =)", "keyword_quote", None);
        expect_no_lint("f(... = 1, `..1` = 2)", "keyword_quote", None);
        expect_no_lint(
            "f(`.1` = 1, `_a` = 2, `1a` = 3, `` = 4)",
            "keyword_quote",
            None,
        );
    }

    #[test]
    fn test_no_lint_keyword_quote_reserved_words() {
        expect_no_lint("c(`next` = 1, 'NA' = 2, `TRUE` = 3)", "keyword_quote", None);
        expect_no_lint("`function` <- 1", "keyword_quote", None);
        expect_no_lint("x$`if`", "keyword_quote", None);
    }

    #[test]
    fn test_no_lint_keyword_quote_assignment() {
        expect_no_lint("`foo bar` <- 1", "keyword_quote", None);
        expect_no_lint("1 -> foo", "keyword_quote", None);
        // data.table assignments
        expect_no_lint("DT[, \"a\" := 1]", "keyword_quote", None);
        // Strings that are not targets
        expect_no_lint("x <- \"foo\"", "keyword_quote", None);
        expect_no_lint("x == \"foo\"", "keyword_quote", None);
    }

    #[test]
    fn test_no_lint_keyword_quote_extraction() {
        expect_no_lint("x$foo", "keyword_quote", None);
        expect_no_lint("x@`foo bar` <- 1", "keyword_quote", None);
        expect_no_lint("x$...", "keyword_quote", None);
        expect_no_lint("x[[\"foo\"]]", "keyword_quote", None);
        expect_no_lint("x[\"foo\"]", "keyword_quote", None);
    }

    #[test]
    fn test_lint_keyword_quote_call() {
        assert_snapshot!(snapshot_lint("c(\"a\" = 1, 'b' = 2, `c` =)\nfoo('a.b_c' = 1)"), @r#"
        warning: keyword_quote
         --> <test>:1:3
          |
        1 | c("a" = 1, 'b' = 2, `c` =)
          |   --- Only quote argument names if necessary.
          |
          = help: Remove the quotes or backticks around the argument name.
        warning: keyword_quote
         --> <test>:1:12
          |
        1 | c("a" = 1, 'b' = 2, `c` =)
          |            --- Only quote argument names if necessary.
          |
          = help: Remove the quotes or backticks around the argument name.
        warning: keyword_quote
         --> <test>:1:21
          |
        1 | c("a" = 1, 'b' = 2, `c` =)
          |                     --- Only quote argument names if necessary.
          |
          = help: Remove the quotes or backticks around the argument name.
        warning: keyword_quote
         --> <test>:2:5
          |
        2 | foo('a.b_c' = 1)
          |     ------- Only quote argument names if necessary.
          |
          = help: Remove the quotes or backticks around the argument name.
        Found 4 errors.
        "#);
    }

    #[test]
    fn test_lint_keyword_quote_assignment() {
        assert_snapshot!(
            snapshot_lint(
                "\"foo bar\" <- 1
'$.my_class' <- function(x, key) { }
'%nin%' <- function(x, table) !x %in% table
1 -> 'a b'
\"foo\" = 1
`foo` <<- 1
1 ->> `foo`"
            ),
            @r#"
        warning: keyword_quote
         --> <test>:1:1
          |
        1 | "foo bar" <- 1
          | --------- Use backticks to create non-syntactic names, not quotes.
          |
          = help: Replace the quotes with backticks.
        warning: keyword_quote
         --> <test>:2:1
          |
        2 | '$.my_class' <- function(x, key) { }
          | ------------ Use backticks to create non-syntactic names, not quotes.
          |
          = help: Replace the quotes with backticks.
        warning: keyword_quote
         --> <test>:3:1
          |
        3 | '%nin%' <- function(x, table) !x %in% table
          | ------- Use backticks to create non-syntactic names, not quotes.
          |
          = help: Replace the quotes with backticks.
        warning: keyword_quote
         --> <test>:4:6
          |
        4 | 1 -> 'a b'
          |      ----- Use backticks to create non-syntactic names, not quotes.
          |
          = help: Replace the quotes with backticks.
        warning: keyword_quote
         --> <test>:5:1
          |
        5 | "foo" = 1
          | ----- Only quote targets of assignment if necessary.
          |
          = help: Remove the quotes or backticks around the name.
        warning: keyword_quote
         --> <test>:6:1
          |
        6 | `foo` <<- 1
          | ----- Only quote targets of assignment if necessary.
          |
          = help: Remove the quotes or backticks around the name.
        warning: keyword_quote
         --> <test>:7:7
          |
        7 | 1 ->> `foo`
          |       ----- Only quote targets of assignment if necessary.
          |
          = help: Remove the quotes or backticks around the name.
        Found 7 errors.
        "#
        );
    }

    #[test]
    fn test_lint_keyword_quote_extraction() {
        assert_snapshot!(
            snapshot_lint(
                "x$\"foo bar\" <- 1
x$\"foo\"
x@`foo`"
            ),
            @r#"
        warning: keyword_quote
         --> <test>:1:3
          |
        1 | x$"foo bar" <- 1
          |   --------- Use backticks to create non-syntactic names, not quotes.
          |
          = help: Replace the quotes with backticks.
        warning: keyword_quote
         --> <test>:2:3
          |
        2 | x$"foo"
          |   ----- Only quote targets of extraction with `$` if necessary.
          |
          = help: Remove the quotes or backticks around the name.
        warning: keyword_quote
         --> <test>:3:3
          |
        3 | x@`foo`
          |   ----- Only quote targets of extraction with `@` if necessary.
          |
          = help: Remove the quotes or backticks around the name.
        Found 3 errors.
        "#
        );
    }

    #[test]
    fn test_keyword_quote_no_fix() {
        assert_snapshot!(
            "no_fix_output",
            get_fixed_text(
                vec!["\"a\\\"b\" <- 1", "'a`b' <- 1", "x$\"a\\nb\"",],
                "keyword_quote",
                None
            )
        );
    }

    #[test]
    fn test_keyword_quote_fix_output() {
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "c(\"a\" = 1, 'b' = 2, `c` = 3, \"d e\" = 4)",
                    "\"foo\" <- 1",
                    "'foo bar' = 1",
                    "1 -> `foo`",
                    "x$\"foo\"",
                    "x@`foo` <- 1",
                    "x$'foo bar'",
                    "r\"(foo)\" <- 1",
                ],
                "keyword_quote",
                None
            )
        );
    }
}
