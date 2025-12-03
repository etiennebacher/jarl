pub(crate) mod sprintf;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_sprintf() {
        expect_no_lint("sprintf('hello %d', 1)", "sprintf", None);
        expect_no_lint("sprintf('hello %d', x)", "sprintf", None);
        expect_no_lint("sprintf('hello %d', x + 1)", "sprintf", None);
        expect_no_lint("sprintf('hello %d', f(x))", "sprintf", None);
        expect_no_lint("sprintf('hello %1$s %1$s', x)", "sprintf", None);
        expect_no_lint("sprintf('hello %1$s %1$s %2$d', x, y)", "sprintf", None);
        expect_no_lint(
            "sprintf('hello %1$s %1$s %2$d %3$s', x, y, 1.5)",
            "sprintf",
            None,
        );
        // Whitespace between "%" and special char is allowed.
        expect_no_lint("sprintf('%   s', 1)", "sprintf", None);
    }

    #[test]
    fn test_lint_sprintf_no_arg() {
        use insta::assert_snapshot;

        let expected_message = "without special characters is useless";

        expect_lint("sprintf('a')", expected_message, "sprintf", None);
        expect_lint("sprintf(\"a\")", expected_message, "sprintf", None);

        assert_snapshot!(
            "fix_output",
            get_fixed_text(vec!["sprintf('a')", "sprintf(\"a\")",], "sprintf", None)
        );
    }

    #[test]
    fn test_lint_sprintf_mismatch() {
        use insta::assert_snapshot;

        let expected_message =
            "Mismatch between number of special characters and number of arguments";

        expect_lint("sprintf('%a')", expected_message, "sprintf", None);
        expect_lint("sprintf('%a %s', 1)", expected_message, "sprintf", None);

        // No fixes because this pattern generates an error at runtime. User
        // needs to fix it.
        assert_snapshot!(
            "no_fix_mismatch",
            get_fixed_text(
                vec!["sprintf('%a')", "sprintf('%a %s', 1)",],
                "sprintf",
                None
            )
        );
    }

    #[test]
    fn test_lint_sprintf_wrong_special_chars() {
        use insta::assert_snapshot;

        let expected_message = "contains some invalid `%`";

        expect_lint("sprintf('%y', 'a')", expected_message, "sprintf", None);
        expect_lint("sprintf('%', 'a')", expected_message, "sprintf", None);
        expect_lint("sprintf('%s%', 'a')", expected_message, "sprintf", None);

        // No fixes because this pattern generates an error at runtime. User
        // needs to fix it.
        assert_snapshot!(
            "no_fix_wrong_special_chars",
            get_fixed_text(vec!["sprintf('%y', 'a')",], "sprintf", None)
        );
    }

    #[test]
    fn test_sprintf_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        expect_lint(
            "sprintf(\n # a comment \n'a')",
            "without special characters is useless",
            "sprintf",
            None,
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nsprintf('a')",
                    "sprintf(\n # a comment \n'a')",
                    "sprintf('a') # trailing comment",
                ],
                "sprintf",
                None
            )
        );
    }
}
