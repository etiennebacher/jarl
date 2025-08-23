pub(crate) mod sample_int;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_sample_int() {
        expect_no_lint("any(x)", "sample_int", None);
        expect_no_lint("duplicated(x)", "sample_int", None);
        expect_no_lint("any(!duplicated(x))", "sample_int", None);
        expect_no_lint("any(!duplicated(foo(x)))", "sample_int", None);
        expect_no_lint("any(na.rm = TRUE)", "sample_int", None);
        expect_no_lint("any()", "sample_int", None);
    }

    #[test]
    fn test_lint_sample_int() {
        use insta::assert_snapshot;

        let expected_message = "`any(duplicated(...))` is inefficient";
        expect_lint("any(duplicated(x))", expected_message, "sample_int", None);
        expect_lint(
            "any(duplicated(foo(x)))",
            expected_message,
            "sample_int",
            None,
        );
        expect_lint(
            "any(duplicated(x), na.rm = TRUE)",
            expected_message,
            "sample_int",
            None,
        );
        expect_lint(
            "any(na.rm = TRUE, duplicated(x))",
            expected_message,
            "sample_int",
            None,
        );
        expect_lint(
            "any(duplicated(x)); 1 + 1; any(duplicated(y))",
            expected_message,
            "sample_int",
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
                "sample_int",
                None
            )
        );
    }
}
