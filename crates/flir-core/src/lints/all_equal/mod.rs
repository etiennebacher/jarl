pub(crate) mod all_equal;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_all_equal() {
        expect_no_lint("any(x)", "all_equal", None);
        expect_no_lint("duplicated(x)", "all_equal", None);
        expect_no_lint("any(!duplicated(x))", "all_equal", None);
        expect_no_lint("any(!duplicated(foo(x)))", "all_equal", None);
        expect_no_lint("any(na.rm = TRUE)", "all_equal", None);
        expect_no_lint("any()", "all_equal", None);
    }

    #[test]
    fn test_lint_all_equal() {
        use insta::assert_snapshot;

        let expected_message = "`any(duplicated(...))` is inefficient";
        expect_lint("any(duplicated(x))", expected_message, "all_equal", None);
        expect_lint(
            "any(duplicated(foo(x)))",
            expected_message,
            "all_equal",
            None,
        );
        expect_lint(
            "any(duplicated(x), na.rm = TRUE)",
            expected_message,
            "all_equal",
            None,
        );
        expect_lint(
            "any(na.rm = TRUE, duplicated(x))",
            expected_message,
            "all_equal",
            None,
        );
        expect_lint(
            "any(duplicated(x)); 1 + 1; any(duplicated(y))",
            expected_message,
            "all_equal",
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
                "all_equal",
                None
            )
        );
    }

    #[test]
    fn test_all_equal_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nany(duplicated(x))",
                    "any(\n  # comment\n  duplicated(x)\n)",
                    "any(duplicated(\n    # comment\n    x\n  ))",
                    "any(duplicated(x)) # trailing comment",
                ],
                "all_equal",
                None
            )
        );
    }
}
