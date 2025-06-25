pub(crate) mod which_grepl;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_which_grepl() {
        use insta::assert_snapshot;
        let expected_message = "`grep(pattern, x)` is better than";

        expect_lint("which(grepl('^a', x))", expected_message, "which_grepl");
        expect_lint(
            "which(grepl('^a', x, perl = TRUE, fixed = TRUE))",
            expected_message,
            "which_grepl",
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "which(grepl('^a', x))",
                    "which(grepl('^a', x, perl = TRUE, fixed = TRUE))",
                ],
                "which_grepl"
            )
        );
    }

    #[test]
    fn test_no_lint_which_grepl() {
        expect_no_lint("which(grepl(p1, x) | grepl(p2, x))", "which_grepl");
        expect_no_lint("which(grep(p1, x))", "which_grepl");
    }
}
