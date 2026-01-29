pub(crate) mod which_grepl;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_which_grepl() {
        use insta::assert_snapshot;
        let expected_message = "Use `grep(pattern, x)` instead";

        expect_lint(
            "which(grepl('^a', x))",
            expected_message,
            "which_grepl",
            None,
        );
        expect_lint(
            "which(grepl('^a', x, perl = TRUE, fixed = TRUE))",
            expected_message,
            "which_grepl",
            None,
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "which(grepl('^a', x))",
                    "which(grepl('^a', x, perl = TRUE, fixed = TRUE))",
                ],
                "which_grepl",
                None
            )
        );
    }

    #[test]
    fn test_no_lint_which_grepl() {
        expect_no_lint("which(grepl(p1, x) | grepl(p2, x))", "which_grepl", None);
        expect_no_lint("which(grep(p1, x))", "which_grepl", None);
    }

    #[test]
    fn test_which_grepl_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nwhich(grepl('^a', x))",
                    "which(\n  # comment\n  grepl('^a', x)\n)",
                    "which(grepl(\n    # comment\n    '^a', x\n  ))",
                    "which(grepl('^a', x)) # trailing comment",
                ],
                "which_grepl",
                None
            )
        );
    }
}
