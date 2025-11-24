pub(crate) mod seq;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_seq() {
        expect_no_lint("1:10", "seq", None);
        expect_no_lint("2:length(x)", "seq", None);
        expect_no_lint("1:(length(x) || 1)", "seq", None);
        expect_no_lint("1:foo(x)", "seq", None);

        // TODO: would be nice to support that
        expect_no_lint("1:dim(x)[1]", "seq", None);
        expect_no_lint("1:dim(x)[[1]]", "seq", None);
    }

    #[test]
    fn test_lint_seq() {
        use insta::assert_snapshot;

        let expected_message = "can be wrong if the RHS is 0";

        expect_lint("1:length(x)", expected_message, "seq", None);
        expect_lint("1:nrow(x)", expected_message, "seq", None);
        expect_lint("1:ncol(x)", expected_message, "seq", None);
        expect_lint("1:NROW(x)", expected_message, "seq", None);
        expect_lint("1:NCOL(x)", expected_message, "seq", None);

        // Same with 1L
        expect_lint("1L:length(x)", expected_message, "seq", None);
        expect_lint("1L:nrow(x)", expected_message, "seq", None);
        expect_lint("1L:ncol(x)", expected_message, "seq", None);
        expect_lint("1L:NROW(x)", expected_message, "seq", None);
        expect_lint("1L:NCOL(x)", expected_message, "seq", None);

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "1:length(x)",
                    "1:nrow(x)",
                    "1:ncol(x)",
                    "1:NROW(x)",
                    "1:NCOL(x)",
                    // Same with 1L
                    "1L:length(x)",
                    "1L:nrow(x)",
                    "1L:ncol(x)",
                    "1L:NROW(x)",
                    "1L:NCOL(x)",
                    "1:length(foo(x))"
                ],
                "seq",
                None
            )
        );
    }

    #[test]
    fn test_seq_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        expect_lint(
            "1:length(\n # a comment \nfoo(x))",
            "can be wrong if the RHS is 0",
            "seq",
            None,
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec!["1:length(\n # a comment \nfoo(x))",],
                "any_is_na",
                None
            )
        );
    }
}
