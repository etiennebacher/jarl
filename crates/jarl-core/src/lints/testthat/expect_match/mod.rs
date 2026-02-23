pub(crate) mod expect_match;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "expect_match", None)
    }

    #[test]
    fn test_no_lint_expect_match() {
        expect_no_lint("grepl('fun', 'Testing is fun')", "expect_match", None);
        expect_no_lint(
            "expect_true(grep('fun', 'Testing is fun'))",
            "expect_match",
            None,
        );
        expect_no_lint(
            "expect_false(grepl('fun', 'Testing is fun'))",
            "expect_match",
            None,
        );
        expect_no_lint("expect_true(is.na(x))", "expect_match", None);
        expect_no_lint("expect_true(grepl())", "expect_match", None);
        expect_no_lint("expect_true(grepl(pattern = 'x'))", "expect_match", None);
        expect_no_lint("expect_true(grepl(x = 'y'))", "expect_match", None);
        expect_no_lint(
            "expect_true(!grepl('hi', 'hello world'))",
            "expect_match",
            None,
        );
        expect_no_lint(
            "!grepl('fun', 'Testing is fun') |> expect_true()",
            "expect_match",
            None,
        );
    }

    #[test]
    fn test_lint_expect_match() {
        assert_snapshot!(snapshot_lint("testthat::expect_true(grepl('fun', 'Testing is fun'))"), @"
        warning: expect_match
         --> <test>:1:1
          |
        1 | testthat::expect_true(grepl('fun', 'Testing is fun'))
          | ----------------------------------------------------- `expect_true(grepl(...))` is not as clear as expect_match(...).
          |
          = help: Use `expect_match(...)` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("show_failure(expect_true(grepl('fun', 'Testing is fun')))"), @"
        warning: expect_match
         --> <test>:1:14
          |
        1 | show_failure(expect_true(grepl('fun', 'Testing is fun')))
          |              ------------------------------------------- `expect_true(grepl(...))` is not as clear as expect_match(...).
          |
          = help: Use `expect_match(...)` instead.
        Found 1 error.
        ");
        assert_snapshot!(
            snapshot_lint("expect_true(grepl('fun', 'Testing is fun'), info = 'msg')"),
            @"
        warning: expect_match
         --> <test>:1:1
          |
        1 | expect_true(grepl('fun', 'Testing is fun'), info = 'msg')
          | --------------------------------------------------------- `expect_true(grepl(...))` is not as clear as expect_match(...).
          |
          = help: Use `expect_match(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_true(grepl('fun', 'Testing is fun'), label = 'lbl')"),
            @"
        warning: expect_match
         --> <test>:1:1
          |
        1 | expect_true(grepl('fun', 'Testing is fun'), label = 'lbl')
          | ---------------------------------------------------------- `expect_true(grepl(...))` is not as clear as expect_match(...).
          |
          = help: Use `expect_match(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_true(grepl(pattern = 'fun', x = 'Testing is fun'))"),
            @"
        warning: expect_match
         --> <test>:1:1
          |
        1 | expect_true(grepl(pattern = 'fun', x = 'Testing is fun'))
          | --------------------------------------------------------- `expect_true(grepl(...))` is not as clear as expect_match(...).
          |
          = help: Use `expect_match(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(snapshot_lint("expect_true(base::grepl('fun', 'Testing is fun'))"), @"
        warning: expect_match
         --> <test>:1:1
          |
        1 | expect_true(base::grepl('fun', 'Testing is fun'))
          | ------------------------------------------------- `expect_true(grepl(...))` is not as clear as expect_match(...).
          |
          = help: Use `expect_match(...)` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("grepl('fun', 'Testing is fun') |> expect_true()"), @"
        warning: expect_match
         --> <test>:1:1
          |
        1 | grepl('fun', 'Testing is fun') |> expect_true()
          | ----------------------------------------------- `expect_true(grepl(...))` is not as clear as expect_match(...).
          |
          = help: Use `expect_match(...)` instead.
        Found 1 error.
        ");
        assert_snapshot!(
            snapshot_lint("'Testing is fun' |> grepl(pattern = 'fun') |> expect_true()"),
            @"
        warning: expect_match
         --> <test>:1:1
          |
        1 | 'Testing is fun' |> grepl(pattern = 'fun') |> expect_true()
          | ----------------------------------------------------------- `expect_true(grepl(...))` is not as clear as expect_match(...).
          |
          = help: Use `expect_match(...)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_fix_expect_match() {
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "expect_true(grepl('fun', 'Testing is fun'))",
                    "testthat::expect_true(grepl('fun', 'Testing is fun'))",
                    "expect_true(grepl(pattern = 'fun', x = 'Testing is fun'))",
                    "expect_true(grepl('fun', 'Testing is fun', perl = TRUE, fixed = FALSE))",
                ],
                "expect_match",
                None,
            )
        );
    }

    #[test]
    fn test_expect_match_extra_args_no_fix() {
        // grepl args should be carried in a fix, no fix for extra expect_true args
        assert_snapshot!(
            "no_fix_extra_args",
            get_fixed_text(
                vec!["expect_true(grepl('fun', 'Testing is fun'), info = 'msg')",],
                "expect_match",
                None,
            )
        );
    }

    #[test]
    fn test_expect_match_with_comments_no_fix() {
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "expect_true(grepl(# comment\n'fun', 'Testing is fun'))",
                    "expect_true(grepl('fun', 'Testing is fun', # comment\nperl = TRUE))",
                ],
                "expect_match",
                None,
            )
        );
    }

    #[test]
    fn test_expect_match_multiline_pipe_no_fix() {
        assert_snapshot!(
            "multiline_pipe",
            get_fixed_text(
                vec![
                    "grepl('fun', 'Testing is fun') |>\n  expect_true()",
                    "'Testing is fun' |>\n  expect_true(grepl(pattern = 'fun'))",
                ],
                "expect_match",
                None,
            )
        );
    }
}
