pub(crate) mod equals_nan;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_equals_nan() {
        use insta::assert_snapshot;

        let expected_message = "Comparing to NaN with";

        expect_lint("x == NaN", expected_message, "equals_nan", None);
        expect_lint("x != NaN", expected_message, "equals_nan", None);
        expect_lint("x %in% NaN", expected_message, "equals_nan", None);
        expect_lint("foo(x(y)) == NaN", expected_message, "equals_nan", None);
        expect_lint("NaN == x", expected_message, "equals_nan", None);

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "x == NaN",
                    "x != NaN",
                    "x %in% NaN",
                    "foo(x(y)) == NaN",
                    "NaN == x",
                ],
                "equals_nan",
                None,
            )
        );
    }

    #[test]
    fn test_no_lint_equals_nan() {
        // `x %in% NaN` returns missings, but `NaN %in% x` returns TRUE/FALSE.
        expect_no_lint("NaN %in% x", "equals_nan", None);

        expect_no_lint("x + NaN", "equals_nan", None);
        expect_no_lint("x == \"NaN\"", "equals_nan", None);
        expect_no_lint("x == 'NaN'", "equals_nan", None);
        expect_no_lint("x <- NaN", "equals_nan", None);
        expect_no_lint("is.nan(x)", "equals_nan", None);
        expect_no_lint("# x == NaN", "equals_nan", None);
        expect_no_lint("'x == NaN'", "equals_nan", None);
        expect_no_lint("x == f(NaN)", "equals_nan", None);
    }

    #[test]
    fn test_equals_nan_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nx == NaN",
                    "x # comment\n== NaN",
                    "x == NaN # trailing comment",
                ],
                "equals_nan",
                None
            )
        );
    }
}
