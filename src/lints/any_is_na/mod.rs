pub(crate) mod any_is_na;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_any_na() {
        use insta::assert_snapshot;

        let expected_message = "`any(is.na(...))` is inefficient";
        expect_lint("any(is.na(x))", expected_message, "any_is_na");
        expect_lint("any(is.na(foo(x)))", expected_message, "any_is_na");
        expect_lint("any(is.na(x), na.rm = TRUE)", expected_message, "any_is_na");
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "any(is.na(x))",
                    "any(is.na(foo(x)))",
                    "any(is.na(x), na.rm = TRUE)",
                ],
                "any_is_na"
            )
        );
    }

    #[test]
    fn test_no_lint_any_na() {
        expect_no_lint("any(x)", "any_is_na");
        expect_no_lint("is.na(x)", "any_is_na");
        expect_no_lint("any(!is.na(x))", "any_is_na");
        expect_no_lint("any(!is.na(foo(x)))", "any_is_na");
        expect_no_lint("any()", "any_is_na");
        expect_no_lint("any(na.rm = TRUE)", "any_is_na");
    }
}
