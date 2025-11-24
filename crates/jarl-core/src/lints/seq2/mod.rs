pub(crate) mod seq2;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_seq2() {
        // seq_len(...) or seq_along(...) expressions are fine
        expect_no_lint("seq_len(x)", "seq2", None);
        expect_no_lint("seq_along(x)", "seq2", None);
        expect_no_lint("seq(2, length(x))", "seq2", None);
        expect_no_lint("seq(length(x), 2)", "seq2", None);
        expect_no_lint("seq()", "seq2", None);
        expect_no_lint("seq(foo(x))", "seq2", None);
    }

    #[test]
    fn test_lint_seq2() {
        use insta::assert_snapshot;

        let expected_message = "can be wrong if the argument has length 0";

        expect_lint("seq(length(x))", expected_message, "seq2", None);
        expect_lint("seq(nrow(x))", expected_message, "seq2", None);
        expect_lint("seq(ncol(x))", expected_message, "seq2", None);
        expect_lint("seq(NROW(x))", expected_message, "seq2", None);
        expect_lint("seq(NCOL(x))", expected_message, "seq2", None);

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "seq(length(x))",
                    "seq(nrow(x))",
                    "seq(ncol(x))",
                    "seq(NROW(x))",
                    "seq(NCOL(x))",
                    "seq(length(foo(x)))"
                ],
                "seq2",
                None
            )
        );
    }

    #[test]
    fn test_seq2_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        expect_lint(
            "seq(length(\n # a comment \nfoo(x)))",
            "can be wrong if the argument has length 0",
            "seq2",
            None,
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec!["seq(length(\n # a comment \nfoo(x)))",],
                "any_is_na",
                None
            )
        );
    }
}
