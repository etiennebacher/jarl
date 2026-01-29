pub(crate) mod repeat;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_repeat() {
        use insta::assert_snapshot;

        let expected_message = "Use `repeat {}` instead";
        expect_lint("while (TRUE) { }", expected_message, "repeat", None);
        expect_lint(
            "for (i in 1:10) { while (TRUE) { if (i == 5) { break } } }",
            expected_message,
            "repeat",
            None,
        );
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "while (TRUE) 1 + 1",
                    "for (i in 1:10) { while (TRUE) { if (i == 5) { break } } }",
                ],
                "repeat",
                None
            )
        );
    }

    #[test]
    fn test_repeat_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(vec!["while (\n#a comment\nTRUE) { }\n",], "any_is_na", None)
        );
    }

    #[test]
    fn test_no_lint_repeat() {
        expect_no_lint("repeat { }", "repeat", None);
        expect_no_lint("while (FALSE) { }", "repeat", None);
        expect_no_lint("while (i < 5) { }", "repeat", None);
        expect_no_lint("while (j < 5) TRUE", "repeat", None);
        expect_no_lint("while (TRUE && j < 5) { ... }", "repeat", None);
    }
}
