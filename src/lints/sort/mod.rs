pub(crate) mod sort;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_sort() {
        expect_no_lint("any(x)", "sort", None);
        expect_no_lint("duplicated(x)", "sort", None);
        expect_no_lint("any(!duplicated(x))", "sort", None);
        expect_no_lint("any(!duplicated(foo(x)))", "sort", None);
        expect_no_lint("any(na.rm = TRUE)", "sort", None);
        expect_no_lint("any()", "sort", None);
    }

    #[test]
    fn test_lint_sort() {
        use insta::assert_snapshot;

        let expected_message = "`any(duplicated(...))` is inefficient";
        expect_lint("any(duplicated(x))", expected_message, "sort", None);
        expect_lint("any(duplicated(foo(x)))", expected_message, "sort", None);
        expect_lint(
            "any(duplicated(x), na.rm = TRUE)",
            expected_message,
            "sort",
            None,
        );
        expect_lint(
            "any(na.rm = TRUE, duplicated(x))",
            expected_message,
            "sort",
            None,
        );
        expect_lint(
            "any(duplicated(x)); 1 + 1; any(duplicated(y))",
            expected_message,
            "sort",
            None,
        );
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "any(duplicated(x))",
                    "any(duplicated(foo(x)))",
                    "any(duplicated(x), na.rm = TRUE)",
                ],
                "sort",
                None
            )
        );
    }
}
