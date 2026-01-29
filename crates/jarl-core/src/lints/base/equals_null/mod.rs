pub(crate) mod equals_null;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_equals_null() {
        use insta::assert_snapshot;

        let expected_message = "Comparing to NULL with";

        expect_lint("x == NULL", expected_message, "equals_null", None);
        expect_lint("x != NULL", expected_message, "equals_null", None);
        expect_lint("x %in% NULL", expected_message, "equals_null", None);
        expect_lint("foo(x(y)) == NULL", expected_message, "equals_null", None);
        expect_lint("NULL == x", expected_message, "equals_null", None);

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "x == NULL",
                    "x != NULL",
                    "x %in% NULL",
                    "foo(x(y)) == NULL",
                    "NULL == x",
                ],
                "equals_null",
                None,
            )
        );
    }

    #[test]
    fn test_no_lint_equals_null() {
        expect_no_lint("x + NULL", "equals_null", None);
        expect_no_lint("x == \"NULL\"", "equals_null", None);
        expect_no_lint("x == 'NULL'", "equals_null", None);
        expect_no_lint("x <- NULL", "equals_null", None);
        expect_no_lint("# x == NULL", "equals_null", None);
        expect_no_lint("'x == NULL'", "equals_null", None);
        expect_no_lint("x == f(NULL)", "equals_null", None);
    }

    #[test]
    fn test_equals_null_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nx == NULL",
                    "x # comment\n== NULL",
                    "x == NULL # trailing comment",
                ],
                "equals_null",
                None
            )
        );
    }
}
