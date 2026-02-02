pub(crate) mod redundant_equals;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_redundant_equals() {
        use insta::assert_snapshot;
        let expected_message = "Using == on a logical vector is";

        expect_lint("a == TRUE", expected_message, "redundant_equals", None);
        expect_lint("TRUE == a", expected_message, "redundant_equals", None);
        expect_lint("a == FALSE", expected_message, "redundant_equals", None);
        expect_lint("FALSE == a", expected_message, "redundant_equals", None);
        expect_lint("a != TRUE", expected_message, "redundant_equals", None);
        expect_lint("TRUE != a", expected_message, "redundant_equals", None);
        expect_lint("a != FALSE", expected_message, "redundant_equals", None);
        expect_lint("FALSE != a", expected_message, "redundant_equals", None);

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "a == TRUE",
                    "TRUE == a",
                    "a == FALSE",
                    "FALSE == a",
                    "a != TRUE",
                    "TRUE != a",
                    "a != FALSE",
                    "FALSE != a",
                    "foo(a(b = 1)) == TRUE"
                ],
                "redundant_equals",
                None
            )
        );
    }

    #[test]
    fn test_no_lint_redundant_equals() {
        expect_no_lint("x == 1", "redundant_equals", None);
        expect_no_lint("x == 'TRUE'", "redundant_equals", None);
        expect_no_lint("x == 'FALSE'", "redundant_equals", None);
        expect_no_lint("x > 1", "redundant_equals", None);
    }

    #[test]
    fn test_redundant_equals_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\na == TRUE",
                    "a # comment\n== TRUE",
                    "# comment\na == FALSE",
                    "a == TRUE # trailing comment",
                ],
                "redundant_equals",
                None
            )
        );
    }
}
