pub(crate) mod expect_length;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_expect_length() {
        use insta::assert_snapshot;
        let expected_message = "`expect_length(x, n)` is better than";

        expect_lint(
            "expect_equal(length(x), 2L)",
            expected_message,
            "expect_length",
        );
        expect_lint(
            "expect_equal(length(x), 2)",
            expected_message,
            "expect_length",
        );
        expect_lint(
            "expect_identical(length(x), 2)",
            expected_message,
            "expect_length",
        );

        // TODO: should work
        // expect_lint(
        //     "expect_equal(2L, length(x))",
        //     expected_message,
        //     "expect_length",
        // );
        // expect_lint(
        //     "expect_equal(2, length(x))",
        //     expected_message,
        //     "expect_length",
        // );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "expect_equal(length(x), 2)",
                    "expect_equal(length(x), 2L)",
                    "expect_identical(length(x), 2)",
                    "expect_equal(2, length(x))",
                ],
                "expect_length"
            )
        );
    }

    #[test]
    fn test_no_lint_expect_length() {
        expect_no_lint("expect_equal(nrow(x), 4L)", "expect_length");
        expect_no_lint(
            "expect_equal(length(x), n, label = 'x size')",
            "expect_length",
        );
        expect_no_lint("expect_equal(length(x), length(y))", "expect_length");
        expect_no_lint(
            "expect_equal(length(x), n, expected.label = 'target size')",
            "expect_length",
        );
        expect_no_lint(
            "expect_equal(length(x), n, info = 'x should have size n')",
            "expect_length",
        );
    }
}
