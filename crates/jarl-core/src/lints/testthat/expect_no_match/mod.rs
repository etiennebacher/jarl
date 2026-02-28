pub(crate) mod expect_no_match;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "expect_no_match", None)
    }

    #[test]
    fn test_no_lint_expect_no_match() {
        expect_no_lint("grepl('fun', 'Testing is fun')", "expect_no_match", None);
        expect_no_lint(
            "expect_false(grep('fun', 'Testing is fun'))",
            "expect_no_match",
            None,
        );
        expect_no_lint(
            "expect_true(grepl('fun', 'Testing is fun'))",
            "expect_no_match",
            None,
        );
        expect_no_lint("expect_false(is.na(x))", "expect_no_match", None);
        expect_no_lint("expect_false(grepl())", "expect_no_match", None);
        expect_no_lint(
            "expect_false(grepl(pattern = 'x'))",
            "expect_no_match",
            None,
        );
        expect_no_lint("expect_false(grepl(x = 'y'))", "expect_no_match", None);
        // Negation cases are handled by `expect_not`, not this rule.
        expect_no_lint(
            "expect_false(!grepl('hi', 'hello world'))",
            "expect_no_match",
            None,
        );
        expect_no_lint(
            "!grepl('fun', 'Testing is fun') |> expect_false()",
            "expect_no_match",
            None,
        );
    }

    #[test]
    fn test_lint_expect_no_match() {
        assert_snapshot!(snapshot_lint("testthat::expect_false(grepl('fun', 'Testing is fun'))"), @r"
        warning: expect_no_match
         --> <test>:1:1
          |
        1 | testthat::expect_false(grepl('fun', 'Testing is fun'))
          | ------------------------------------------------------ `expect_false(grepl(...))` is not as clear as `expect_no_match(...)`.
          |
          = help: Use `expect_no_match(...)` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("show_failure(expect_false(grepl('fun', 'Testing is fun')))"), @r"
        warning: expect_no_match
         --> <test>:1:14
          |
        1 | show_failure(expect_false(grepl('fun', 'Testing is fun')))
          |              -------------------------------------------- `expect_false(grepl(...))` is not as clear as `expect_no_match(...)`.
          |
          = help: Use `expect_no_match(...)` instead.
        Found 1 error.
        ");
        assert_snapshot!(
            snapshot_lint("expect_false(grepl('fun', 'Testing is fun'), info = 'msg')"),
            @r"
        warning: expect_no_match
         --> <test>:1:1
          |
        1 | expect_false(grepl('fun', 'Testing is fun'), info = 'msg')
          | ---------------------------------------------------------- `expect_false(grepl(...))` is not as clear as `expect_no_match(...)`.
          |
          = help: Use `expect_no_match(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_false(grepl(pattern = 'fun', x = 'Testing is fun'))"),
            @r"
        warning: expect_no_match
         --> <test>:1:1
          |
        1 | expect_false(grepl(pattern = 'fun', x = 'Testing is fun'))
          | ---------------------------------------------------------- `expect_false(grepl(...))` is not as clear as `expect_no_match(...)`.
          |
          = help: Use `expect_no_match(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_false(grepl(x = 'Testing is fun', perl = TRUE, pattern = 'fun'))"),
            @r"
        warning: expect_no_match
         --> <test>:1:1
          |
        1 | expect_false(grepl(x = 'Testing is fun', perl = TRUE, pattern = 'fun'))
          | ----------------------------------------------------------------------- `expect_false(grepl(...))` is not as clear as `expect_no_match(...)`.
          |
          = help: Use `expect_no_match(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(snapshot_lint("expect_false(base::grepl('fun', 'Testing is fun'))"), @r"
        warning: expect_no_match
         --> <test>:1:1
          |
        1 | expect_false(base::grepl('fun', 'Testing is fun'))
          | -------------------------------------------------- `expect_false(grepl(...))` is not as clear as `expect_no_match(...)`.
          |
          = help: Use `expect_no_match(...)` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("grepl('fun', 'Testing is fun') |> expect_false()"), @r"
        warning: expect_no_match
         --> <test>:1:1
          |
        1 | grepl('fun', 'Testing is fun') |> expect_false()
          | ------------------------------------------------ `expect_false(grepl(...))` is not as clear as `expect_no_match(...)`.
          |
          = help: Use `expect_no_match(...)` instead.
        Found 1 error.
        ");
        assert_snapshot!(
            snapshot_lint("'Testing is fun' |> grepl(pattern = 'fun') |> expect_false()"),
            @r"
        warning: expect_no_match
         --> <test>:1:1
          |
        1 | 'Testing is fun' |> grepl(pattern = 'fun') |> expect_false()
          | ------------------------------------------------------------ `expect_false(grepl(...))` is not as clear as `expect_no_match(...)`.
          |
          = help: Use `expect_no_match(...)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_fix_expect_no_match() {
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "expect_false(grepl('fun', 'Testing is fun'))",
                    "testthat::expect_false(grepl('fun', 'Testing is fun'))",
                    "expect_false(grepl(pattern = 'fun', x = 'Testing is fun'))",
                    "expect_false(grepl(x = 'Testing is fun', perl = TRUE, pattern = 'fun'))",
                    "expect_false(grepl('fun', 'Testing is fun', perl = TRUE, fixed = FALSE))",
                ],
                "expect_no_match",
                None,
            )
        );
    }

    #[test]
    fn test_expect_no_match_extra_args_no_fix() {
        // grepl args should be carried in a fix, no fix for:
        // - extra expect_false args
        // - positional optional grepl args (unsafe to rewrite)
        assert_snapshot!(
            "no_fix_extra_args",
            get_fixed_text(
                vec![
                    "expect_false(grepl('fun', 'Testing is fun'), info = 'msg')",
                    "expect_false(grepl('fun', 'Testing is fun', FALSE, FALSE, FALSE, FALSE))",
                ],
                "expect_no_match",
                None,
            )
        );
    }

    #[test]
    fn test_expect_no_match_with_comments_no_fix() {
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec!["expect_false(grepl(# comment\n'fun', 'Testing is fun'))",],
                "expect_no_match",
                None,
            )
        );
    }

    #[test]
    fn test_expect_no_match_multiline_pipe_no_fix() {
        assert_snapshot!(
            "multiline_pipe",
            get_fixed_text(
                vec![
                    "grepl('fun', 'Testing is fun') |>\n  expect_false()",
                    "'Testing is fun' |>\n  expect_false(grepl(pattern = 'fun'))",
                ],
                "expect_no_match",
                None,
            )
        );
    }
}
