pub(crate) mod repeat;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "repeat", None)
    }

    #[test]
    fn test_lint_repeat() {
        assert_snapshot!(
            snapshot_lint("while (TRUE) { }"),
            @r"
        warning: repeat
         --> <test>:1:1
          |
        1 | while (TRUE) { }
          | ------------ `while (TRUE)` is less clear than `repeat` for infinite loops.
          |
          = help: Use `repeat {}` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (i in 1:10) { while (TRUE) { if (i == 5) { break } } }"),
            @r"
        warning: repeat
         --> <test>:1:19
          |
        1 | for (i in 1:10) { while (TRUE) { if (i == 5) { break } } }
          |                   ------------ `while (TRUE)` is less clear than `repeat` for infinite loops.
          |
          = help: Use `repeat {}` instead.
        Found 1 error.
        "
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
