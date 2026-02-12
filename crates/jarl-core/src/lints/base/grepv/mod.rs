pub(crate) mod grepv;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "grepv", Some("4.5"))
    }

    #[test]
    fn test_no_lint_grepv() {
        expect_no_lint("grep('i', x)", "grepv", Some("4.5"));
        expect_no_lint("grep(pattern = 'i', x)", "grepv", Some("4.5"));
        expect_no_lint("grep('i', x, TRUE, TRUE)", "grepv", Some("4.5"));
    }

    #[test]
    fn test_lint_grepv() {
        assert_snapshot!(
            snapshot_lint("grep('i', x, value = TRUE)"),
            @r"
        warning: grepv
         --> <test>:1:1
          |
        1 | grep('i', x, value = TRUE)
          | -------------------------- `grep(..., value = TRUE)` can be simplified.
          |
          = help: Use `grepv(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("grep('i', x, TRUE, TRUE, TRUE)"),
            @r"
        warning: grepv
         --> <test>:1:1
          |
        1 | grep('i', x, TRUE, TRUE, TRUE)
          | ------------------------------ `grep(..., value = TRUE)` can be simplified.
          |
          = help: Use `grepv(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("grep('i', x, TRUE, TRUE, TRUE, value = TRUE)"),
            @r"
        warning: grepv
         --> <test>:1:1
          |
        1 | grep('i', x, TRUE, TRUE, TRUE, value = TRUE)
          | -------------------------------------------- `grep(..., value = TRUE)` can be simplified.
          |
          = help: Use `grepv(...)` instead.
        Found 1 error.
        "
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
