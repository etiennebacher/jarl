pub(crate) mod expect_true_false;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

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
        use insta::assert_snapshot;
        let expected_message = "not as clear as";

        expect_lint(
            "expect_equal(foo(x), TRUE)",
            expected_message,
            "expect_true_false",
            None,
        );
        expect_lint(
            "expect_equal(TRUE, foo(x))",
            expected_message,
            "expect_true_false",
            None,
        );
        expect_lint(
            "expect_identical(x, FALSE)",
            expected_message,
            "expect_true_false",
            None,
        );
        expect_lint(
            "expect_identical(FALSE, x)",
            expected_message,
            "expect_true_false",
            None,
        );
        expect_lint(
            "expect_equal(is.numeric(x), FALSE)",
            expected_message,
            "expect_true_false",
            None,
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
