pub(crate) mod system_file;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "system_file", None)
    }

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
        assert_snapshot!(
            snapshot_lint("system.file(file.path('path', 'to', 'data'), package = 'foo')"),
            @r"
        warning: system_file
         --> <test>:1:1
          |
        1 | system.file(file.path('path', 'to', 'data'), package = 'foo')
          | ------------------------------------------------------------- `system.file(file.path(...))` is redundant.
          |
          = help: Use `system.file(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("base::system.file(file.path('path', 'to', 'data'), package = 'foo')"),
            @r"
        warning: system_file
         --> <test>:1:1
          |
        1 | base::system.file(file.path('path', 'to', 'data'), package = 'foo')
          | ------------------------------------------------------------------- `system.file(file.path(...))` is redundant.
          |
          = help: Use `system.file(...)` instead.
        Found 1 error.
        "
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
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            snapshot_lint("system.file(\n # a comment\nfile.path('path', 'to', 'data'), package = 'foo')"),
            @r"
        warning: system_file
         --> <test>:1:1
          |
        1 | / system.file(
        2 | |  # a comment
        3 | | file.path('path', 'to', 'data'), package = 'foo')
          | |_________________________________________________- `system.file(file.path(...))` is redundant.
          |
          = help: Use `system.file(...)` instead.
        Found 1 error.
        "
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
