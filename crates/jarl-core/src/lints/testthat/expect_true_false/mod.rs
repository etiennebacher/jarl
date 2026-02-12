pub(crate) mod expect_true_false;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "expect_true_false", None)
    }

    #[test]
    fn test_no_lint_expect_true_false() {
        expect_no_lint("expect_equal(x, 1)", "expect_true_false", None);
        expect_no_lint("expect_equal(x, 'TRUE')", "expect_true_false", None);
        expect_no_lint("foo(x, TRUE)", "expect_true_false", None);

        // expect_true cannot test logical vectors
        expect_no_lint("expect_equal(x, c(TRUE, FALSE))", "expect_true_false", None);
        expect_no_lint("expect_equal(c(TRUE, FALSE), x)", "expect_true_false", None);
    }

    #[test]
    fn test_lint_expect_true_false() {
        assert_snapshot!(
            snapshot_lint("expect_equal(foo(x), TRUE)"),
            @r"
        warning: expect_true_false
         --> <test>:1:1
          |
        1 | expect_equal(foo(x), TRUE)
          | -------------------------- `expect_equal(x, TRUE)` is not as clear as `expect_true(x)`.
          |
          = help: Use `expect_true(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("testthat::expect_equal(foo(x), TRUE)"),
            @r"
        warning: expect_true_false
         --> <test>:1:1
          |
        1 | testthat::expect_equal(foo(x), TRUE)
          | ------------------------------------ `expect_equal(x, TRUE)` is not as clear as `expect_true(x)`.
          |
          = help: Use `expect_true(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_equal(TRUE, foo(x))"),
            @r"
        warning: expect_true_false
         --> <test>:1:1
          |
        1 | expect_equal(TRUE, foo(x))
          | -------------------------- `expect_equal(x, TRUE)` is not as clear as `expect_true(x)`.
          |
          = help: Use `expect_true(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_identical(x, FALSE)"),
            @r"
        warning: expect_true_false
         --> <test>:1:1
          |
        1 | expect_identical(x, FALSE)
          | -------------------------- `expect_identical(x, FALSE)` is not as clear as `expect_false(x)`.
          |
          = help: Use `expect_false(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_identical(FALSE, x)"),
            @r"
        warning: expect_true_false
         --> <test>:1:1
          |
        1 | expect_identical(FALSE, x)
          | -------------------------- `expect_identical(x, FALSE)` is not as clear as `expect_false(x)`.
          |
          = help: Use `expect_false(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_equal(is.numeric(x), FALSE)"),
            @r"
        warning: expect_true_false
         --> <test>:1:1
          |
        1 | expect_equal(is.numeric(x), FALSE)
          | ---------------------------------- `expect_equal(x, FALSE)` is not as clear as `expect_false(x)`.
          |
          = help: Use `expect_false(x)` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "expect_equal(foo(x), TRUE)",
                    "expect_equal(foo(x), FALSE)",
                    "expect_identical(foo(x), TRUE)",
                    "expect_identical(foo(x), FALSE)",
                    "expect_equal(TRUE, foo(x))",
                    "expect_equal(FALSE, foo(x))",
                    "testthat::expect_equal(x, TRUE)",
                ],
                "expect_true_false",
                None,
            )
        );
    }

    #[test]
    fn test_expect_true_false_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nexpect_equal(x, TRUE)",
                    "expect_equal(x, # comment\nTRUE)",
                    "expect_equal(x, TRUE) # trailing comment",
                ],
                "expect_true_false",
                None
            )
        );
    }
}
