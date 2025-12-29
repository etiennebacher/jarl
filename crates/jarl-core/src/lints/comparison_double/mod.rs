pub(crate) mod comparison_double;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_comparison_double() {
        expect_no_lint("2 == 2", "comparison_double", None);
        expect_no_lint("x == x", "comparison_double", None);
        expect_no_lint("x == 1L", "comparison_double", None);
        expect_no_lint("'a' == x", "comparison_double", None);
        expect_no_lint("x == c(1, 2)", "comparison_double", None);
    }

    #[test]
    fn test_lint_comparison_double() {
        use insta::assert_snapshot;
        let message = "Comparing to a double can lead to unexpected results";

        expect_lint("x == 2", message, "comparison_double", None);
        expect_lint("x == 2.1", message, "comparison_double", None);
        expect_lint("1 == x", message, "comparison_double", None);
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec!["x == 2", "x == 2.1", "1 == x",],
                "comparison_double",
                None
            )
        );
    }

    #[test]
    fn test_comparison_double_with_comments_no_fix() {
        use insta::assert_snapshot;
        let message = "Comparing to a double can lead to unexpected results";

        expect_lint(
            "# leading comment\nx == 2",
            message,
            "comparison_double",
            None,
        );
        expect_lint(
            "x == \n # hello there \n 2",
            message,
            "comparison_double",
            None,
        );
        expect_lint(
            "x == 2 # trailing comment",
            message,
            "comparison_double",
            None,
        );

        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nx == 2",
                    "x == \n # hello there \n 2",
                    "x == 2 # trailing comment",
                ],
                "comparison_double",
                None
            )
        );
    }
}
