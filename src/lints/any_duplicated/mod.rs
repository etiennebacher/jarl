pub(crate) mod any_duplicated;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_any_duplicated() {
        expect_no_lint("any(x)", "any_duplicated");
        expect_no_lint("duplicated(x)", "any_duplicated");
        expect_no_lint("any(!duplicated(x))", "any_duplicated");
        expect_no_lint("any(!duplicated(foo(x)))", "any_duplicated");
        expect_no_lint("any(na.rm = TRUE)", "any_duplicated");
        expect_no_lint("any()", "any_duplicated");
    }

    #[test]
    fn test_lint_any_duplicated() {
        use insta::assert_snapshot;

        let expected_message = "`any(duplicated(...))` is inefficient";
        expect_lint("any(duplicated(x))", expected_message, "any_duplicated");
        expect_lint(
            "any(duplicated(foo(x)))",
            expected_message,
            "any_duplicated",
        );
        expect_lint(
            "any(duplicated(x), na.rm = TRUE)",
            expected_message,
            "any_duplicated",
        );
        expect_lint(
            "any(na.rm = TRUE, duplicated(x))",
            expected_message,
            "any_duplicated",
        );
        expect_lint(
            "any(duplicated(x)); 1 + 1; any(duplicated(y))",
            expected_message,
            "any_duplicated",
        );
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "any(duplicated(x))",
                    "any(duplicated(foo(x)))",
                    "any(duplicated(x), na.rm = TRUE)",
                ],
                "any_duplicated",
            )
        );
    }
}
