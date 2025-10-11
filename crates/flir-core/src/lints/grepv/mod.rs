pub(crate) mod grepv;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_grepv() {
        expect_no_lint("grep('i', x)", "grepv", Some("4.5"));
        expect_no_lint("grep(pattern = 'i', x)", "grepv", Some("4.5"));
        expect_no_lint("grep('i', x, TRUE, TRUE)", "grepv", Some("4.5"));
    }

    #[test]
    fn test_lint_grepv() {
        use insta::assert_snapshot;

        let expected_message = "Use `grepv(...)`";
        has_lint(
            "grep('i', x, value = TRUE)",
            expected_message,
            "grepv",
            Some("4.5"),
        );
        has_lint(
            "grep('i', x, TRUE, TRUE, TRUE)",
            expected_message,
            "grepv",
            Some("4.5"),
        );
        has_lint(
            "grep('i', x, TRUE, TRUE, TRUE, value = TRUE)",
            expected_message,
            "grepv",
            Some("4.5"),
        );
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "grep('i', x, value = TRUE)",
                    "grep('i', x, TRUE, TRUE, TRUE)",
                    "grep('i', x, TRUE, TRUE, TRUE, value = TRUE)",
                    // Keep the name of other args
                    "grep(pattern = 'i', x, value = TRUE)",
                    // Wrong code but no panic
                    "grep(value = TRUE)",
                ],
                "grepv",
                Some("4.5")
            )
        );
    }

    #[test]
    fn test_grepv_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\ngrep('i', x, value = TRUE)",
                    "grep(\n  # comment\n  'i', x, value = TRUE\n)",
                    "grep('i',\n    # comment\n    x, value = TRUE)",
                    "grep('i', x, value = TRUE) # trailing comment",
                ],
                "grepv",
                Some("4.5")
            )
        );
    }
}
