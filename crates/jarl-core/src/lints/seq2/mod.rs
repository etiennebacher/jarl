pub(crate) mod seq2;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_seq2() {
        use insta::assert_snapshot;

        let expected_message = "Use `seq2 {}` instead";
        expect_lint("while (TRUE) { }", expected_message, "seq2", None);
        expect_lint(
            "for (i in 1:10) { while (TRUE) { if (i == 5) { break } } }",
            expected_message,
            "seq2",
            None,
        );
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "while (TRUE) 1 + 1",
                    "for (i in 1:10) { while (TRUE) { if (i == 5) { break } } }",
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
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(vec!["while (\n#a comment\nTRUE) { }\n",], "any_is_na", None)
        );
    }

    #[test]
    fn test_no_lint_seq2() {
        expect_no_lint("seq2 { }", "seq2", None);
        expect_no_lint("while (FALSE) { }", "seq2", None);
        expect_no_lint("while (i < 5) { }", "seq2", None);
        expect_no_lint("while (j < 5) TRUE", "seq2", None);
        expect_no_lint("while (TRUE && j < 5) { ... }", "seq2", None);
    }
}
