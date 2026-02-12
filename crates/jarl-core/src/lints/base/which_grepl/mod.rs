pub(crate) mod which_grepl;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "which_grepl", None)
    }

    #[test]
    fn test_lint_which_grepl() {
        assert_snapshot!(
            snapshot_lint("which(grepl('^a', x))"),
            @r"
        warning: which_grepl
         --> <test>:1:1
          |
        1 | which(grepl('^a', x))
          | --------------------- `which(grepl(pattern, x))` is less efficient than `grep(pattern, x)`.
          |
          = help: Use `grep(pattern, x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("which(grepl('^a', x, perl = TRUE, fixed = TRUE))"),
            @r"
        warning: which_grepl
         --> <test>:1:1
          |
        1 | which(grepl('^a', x, perl = TRUE, fixed = TRUE))
          | ------------------------------------------------ `which(grepl(pattern, x))` is less efficient than `grep(pattern, x)`.
          |
          = help: Use `grep(pattern, x)` instead.
        Found 1 error.
        "
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
