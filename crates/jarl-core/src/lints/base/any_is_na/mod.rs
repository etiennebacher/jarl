pub(crate) mod any_is_na;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_any_na() {
        expect_no_lint("any(x)", "any_is_na", None);
        expect_no_lint("is.na(x)", "any_is_na", None);
        expect_no_lint("any(!is.na(x))", "any_is_na", None);
        expect_no_lint("any(!is.na(foo(x)))", "any_is_na", None);
        expect_no_lint("any()", "any_is_na", None);
        expect_no_lint("any(na.rm = TRUE)", "any_is_na", None);
    }

    #[test]
    fn test_lint_any_na() {
        use insta::assert_snapshot;

        let expected_message = "`any(is.na(...))` is inefficient";
        expect_lint("any(is.na(x))", expected_message, "any_is_na", None);
        expect_lint("any(is.na(foo(x)))", expected_message, "any_is_na", None);
        expect_lint(
            "base::any(is.na(foo(x)))",
            expected_message,
            "any_is_na",
            None,
        );
        expect_lint(
            "any(is.na(x), na.rm = TRUE)",
            expected_message,
            "any_is_na",
            None,
        );

        expect_lint(
            "NA %in% x",
            "`NA %in% x` is inefficient.",
            "any_is_na",
            None,
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "any(is.na(x))",
                    "NA %in% x",
                    "any(is.na(foo(x)))",
                    "any(is.na(x), na.rm = TRUE)",
                ],
                "any_is_na",
                None
            )
        );
    }

    #[test]
    fn test_any_is_na_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nany(is.na(x))",
                    "any(\n  # comment\n  is.na(x)\n)",
                    "any(is.na(\n    # comment\n    x\n  ))",
                    "any(is.na(x)) # trailing comment",
                ],
                "any_is_na",
                None
            )
        );
    }
}
