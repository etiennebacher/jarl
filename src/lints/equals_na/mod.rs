pub(crate) mod equals_na;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_equals_na() {
        use insta::assert_snapshot;

        let expected_message = "instead of comparing to NA";

        expect_lint("x == NA", expected_message, "equals_na");
        expect_lint("x == NA_integer_", expected_message, "equals_na");
        expect_lint("x == NA_real_", expected_message, "equals_na");
        expect_lint("x == NA_logical_", expected_message, "equals_na");
        expect_lint("x == NA_character_", expected_message, "equals_na");
        expect_lint("x == NA_complex_", expected_message, "equals_na");
        expect_lint("x != NA", expected_message, "equals_na");
        expect_lint("foo(x(y)) == NA", expected_message, "equals_na");
        expect_lint("NA == x", expected_message, "equals_na");

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
            )
        );
    }

    #[test]
    fn test_no_lint_equals_na() {
        expect_no_lint("x + NA", "equals_na");
        expect_no_lint("x == \"NA\"", "equals_na");
        expect_no_lint("x == 'NA'", "equals_na");
        expect_no_lint("x <- NA", "equals_na");
        expect_no_lint("x <- NaN", "equals_na");
        expect_no_lint("x <- NA_real_", "equals_na");
        expect_no_lint("is.na(x)", "equals_na");
        expect_no_lint("is.nan(x)", "equals_na");
        expect_no_lint("x[!is.na(x)]", "equals_na");
        expect_no_lint("# x == NA", "equals_na");
        expect_no_lint("'x == NA'", "equals_na");
        expect_no_lint("x == f(NA)", "equals_na");
    }
}
