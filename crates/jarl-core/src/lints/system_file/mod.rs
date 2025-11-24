pub(crate) mod system_file;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_system_file() {
        expect_no_lint("system.file('a', 'b', 'c')", "system_file", None);
        expect_no_lint("system.file(paste('a', 'b', 'c'))", "system_file", None);
        expect_no_lint("system.file(file.path())", "system_file", None);
        expect_no_lint("system.file(file.path(,))", "system_file", None);
        expect_no_lint("file.path('a', 'b', 'c')", "system_file", None);
    }

    #[test]
    fn test_lint_system_file() {
        use insta::assert_snapshot;
        let expected_message = "system.file(file.path(...))` is redundant.";

        expect_lint(
            "system.file(file.path('path', 'to', 'data'), package = 'foo')",
            expected_message,
            "system_file",
            None,
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec!["system.file(file.path('path', 'to', 'data'), package = 'foo')",],
                "system_file",
                None
            )
        );
    }

    #[test]
    fn test_system_file_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        expect_lint(
            "system.file(\n # a comment\nfile.path('path', 'to', 'data'), package = 'foo')",
            "system.file(file.path(...))` is redundant.",
            "system_file",
            None,
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nsystem.file(file.path('path', 'to', 'data'), package = 'foo')",
                    "system.file(\n # a comment\nfile.path('path', 'to', 'data'), package = 'foo')",
                    "system.file(file.path('path', 'to', 'data'), package = 'foo') # trailing comment",
                ],
                "system_file",
                None
            )
        );
    }
}
