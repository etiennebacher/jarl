pub(crate) mod expect_true_false;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_expect_true_false() {
        // expect_true is a scalar test; testing logical vectors with expect_equal is OK
        expect_no_lint("expect_equal(x, c(TRUE, FALSE))", "expect_true_false", None);

        // Not the functions we're looking for
        expect_no_lint("expect_equal(x, 1)", "expect_true_false", None);
        expect_no_lint("expect_equal(x, 'TRUE')", "expect_true_false", None);
        expect_no_lint("some_other_function(x, TRUE)", "expect_true_false", None);
    }

    #[test]
    fn test_lint_expect_true_false() {
        let expected_message = "Use `expect_true()` or `expect_false()`";

        // expect_equal with TRUE
        expect_lint(
            "expect_equal(foo(x), TRUE)",
            expected_message,
            "expect_true_false",
            None,
        );

        // expect_equal with TRUE as first argument
        expect_lint(
            "expect_equal(TRUE, foo(x))",
            expected_message,
            "expect_true_false",
            None,
        );

        // expect_identical with FALSE
        expect_lint(
            "expect_identical(x, FALSE)",
            expected_message,
            "expect_true_false",
            None,
        );

        // expect_identical with FALSE as first argument
        expect_lint(
            "expect_identical(FALSE, x)",
            expected_message,
            "expect_true_false",
            None,
        );

        // expect_equal with FALSE
        expect_lint(
            "expect_equal(is.numeric(x), FALSE)",
            expected_message,
            "expect_true_false",
            None,
        );
    }

    #[test]
    fn test_fix_expect_true_false() {
        use insta::assert_snapshot;

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
                ],
                "expect_true_false",
                None,
            )
        );
    }

    #[test]
    fn test_expect_true_false_with_comments_no_fix() {
        use insta::assert_snapshot;
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
