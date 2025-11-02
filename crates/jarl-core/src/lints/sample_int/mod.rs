pub(crate) mod sample_int;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_sample_int() {
        expect_no_lint("sample('a', m)", "sample_int", None);
        expect_no_lint("sample(1, m)", "sample_int", None);
        expect_no_lint("sample(n, m)", "sample_int", None);
        expect_no_lint("sample(n, m, TRUE)", "sample_int", None);
        expect_no_lint("sample(n, m, prob = 1:n/n)", "sample_int", None);
        expect_no_lint("sample(foo(x), m, TRUE)", "sample_int", None);
        expect_no_lint("sample(n, replace = TRUE)", "sample_int", None);
        expect_no_lint("sample(10:1, m)", "sample_int", None);
        expect_no_lint("sample(replace = TRUE, letters)", "sample_int", None);
        expect_no_lint("x$sample(1:2, 1)", "sample_int", None);
    }

    #[test]
    fn test_lint_sample_int() {
        use insta::assert_snapshot;

        let expected_message = "is less readable than `sample.int";
        expect_lint("sample(1:10, 2)", expected_message, "sample_int", None);
        expect_lint("sample(1L:10L, 2)", expected_message, "sample_int", None);
        expect_lint("sample(1:n, 2)", expected_message, "sample_int", None);
        expect_lint(
            "sample(1:k, replace = TRUE)",
            expected_message,
            "sample_int",
            None,
        );
        expect_lint(
            "sample(1:foo(x), prob = bar(x))",
            expected_message,
            "sample_int",
            None,
        );
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "sample(1:10, 2)",
                    "sample(1L:10L, 2)",
                    "sample(n = 1:10, 2)",
                    "sample(2, n = 1:10)",
                    "sample(size = 2, n = 1:10)",
                    "sample(replace = TRUE, letters)",
                ],
                "sample_int",
                None
            )
        );
    }

    #[test]
    fn test_sample_int_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nsample(1:10, 2)",
                    "sample(\n  # comment\n  1:10, 2\n)",
                    "sample(1:n,\n    # comment\n    2)",
                    "sample(1:10, 2) # trailing comment",
                ],
                "sample_int",
                None
            )
        );
    }
}
