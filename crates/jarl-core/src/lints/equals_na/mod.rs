pub(crate) mod equals_na;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_equals_na() {
        use insta::assert_snapshot;

        let expected_message = "Comparing to NA with";

        expect_lint("x == NA", expected_message, "equals_na", None);
        expect_lint("x == NA_integer_", expected_message, "equals_na", None);
        expect_lint("x == NA_real_", expected_message, "equals_na", None);
        expect_lint("x == NA_logical_", expected_message, "equals_na", None);
        expect_lint("x == NA_character_", expected_message, "equals_na", None);
        expect_lint("x == NA_complex_", expected_message, "equals_na", None);
        expect_lint("x != NA", expected_message, "equals_na", None);
        expect_lint("foo(x(y)) == NA", expected_message, "equals_na", None);
        expect_lint("NA == x", expected_message, "equals_na", None);

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "x == NA",
                    "x == NA_integer_",
                    "x == NA_real_",
                    "x == NA_logical_",
                    "x == NA_character_",
                    "x == NA_complex_",
                    "x != NA",
                    "foo(x(y)) == NA",
                    "NA == x",
                ],
                "equals_na",
                None,
            )
        );
    }

    #[test]
    fn test_no_lint_equals_na() {
        expect_no_lint("x + NA", "equals_na", None);
        expect_no_lint("x == \"NA\"", "equals_na", None);
        expect_no_lint("x == 'NA'", "equals_na", None);
        expect_no_lint("x <- NA", "equals_na", None);
        expect_no_lint("x <- NaN", "equals_na", None);
        expect_no_lint("x <- NA_real_", "equals_na", None);
        expect_no_lint("is.na(x)", "equals_na", None);
        expect_no_lint("is.nan(x)", "equals_na", None);
        expect_no_lint("x[!is.na(x)]", "equals_na", None);
        expect_no_lint("# x == NA", "equals_na", None);
        expect_no_lint("'x == NA'", "equals_na", None);
        expect_no_lint("x == f(NA)", "equals_na", None);
    }

    #[test]
    fn test_equals_na_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nx == NA",
                    "x # comment\n== NA",
                    "# comment\nx == NA",
                    "x == NA # trailing comment",
                ],
                "equals_na",
                None
            )
        );
    }
}
