pub(crate) mod expect_null;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "expect_null", None)
    }

    #[test]
    fn test_no_lint_expect_null() {
        expect_no_lint("expect_true(!is.null(x))", "expect_null", None);
        expect_no_lint("testthat::expect_true(!is.null(x))", "expect_null", None);
        expect_no_lint("expect_equal()", "expect_null", None);
        expect_no_lint("expect_true()", "expect_null", None);

        // length-0 could be NULL, but could be integer() or list(), so let it pass
        expect_no_lint("expect_length(x, 0L)", "expect_null", None);

        // no false positive for is.null() at the wrong positional argument
        expect_no_lint("expect_true(x, is.null(y))", "expect_null", None);

        // Not the functions we're looking for
        expect_no_lint("expect_equal(x, 1)", "expect_null", None);
        expect_no_lint("some_other_function(x, NULL)", "expect_null", None);
        expect_no_lint("expect_true(foo(x))", "expect_null", None);

        // Wrong code but no panic
        expect_no_lint("expect_equal(object =, NULL)", "expect_null", None);
        expect_no_lint("expect_equal(x, expected =)", "expect_null", None);
        expect_no_lint("expect_equal(object = x)", "expect_null", None);

        expect_no_lint("expect_true(object =)", "expect_null", None);
        expect_no_lint("expect_true(is.null())", "expect_null", None);
        expect_no_lint("expect_true(is.null(x =))", "expect_null", None);

        expect_no_lint("expect_equal(expected = NULL)", "expect_null", None);
    }

    #[test]
    fn test_lint_expect_null() {
        assert_snapshot!(
            snapshot_lint("expect_equal(x, NULL)"),
            @r"
        warning: expect_null
         --> <test>:1:1
          |
        1 | expect_equal(x, NULL)
          | --------------------- `expect_equal(x, NULL)` is not as clear as `expect_null(x)`.
          |
          = help: Use `expect_null(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("testthat::expect_equal(x, NULL)"),
            @r"
        warning: expect_null
         --> <test>:1:1
          |
        1 | testthat::expect_equal(x, NULL)
          | ------------------------------- `expect_equal(x, NULL)` is not as clear as `expect_null(x)`.
          |
          = help: Use `expect_null(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_identical(x, NULL)"),
            @r"
        warning: expect_null
         --> <test>:1:1
          |
        1 | expect_identical(x, NULL)
          | ------------------------- `expect_identical(x, NULL)` is not as clear as `expect_null(x)`.
          |
          = help: Use `expect_null(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_equal(NULL, x)"),
            @r"
        warning: expect_null
         --> <test>:1:1
          |
        1 | expect_equal(NULL, x)
          | --------------------- `expect_equal(x, NULL)` is not as clear as `expect_null(x)`.
          |
          = help: Use `expect_null(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_true(is.null(foo(x)))"),
            @r"
        warning: expect_null
         --> <test>:1:1
          |
        1 | expect_true(is.null(foo(x)))
          | ---------------------------- `expect_true(is.null(x))` is not as clear as `expect_null(x)`.
          |
          = help: Use `expect_null(x)` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "expect_equal(x, NULL)",
                    "expect_identical(x, NULL)",
                    "expect_equal(NULL, x)",
                    "expect_true(is.null(foo(x)))",
                    "testthat::expect_true(base::is.null(x))",
                ],
                "expect_null",
                None
            )
        );
    }

    #[test]
    fn test_expect_null_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nexpect_equal(x, NULL)",
                    "expect_equal(x, # comment\nNULL)",
                    "expect_equal(x, NULL) # trailing comment",
                ],
                "expect_null",
                None
            )
        );
    }
}
