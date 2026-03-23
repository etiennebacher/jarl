pub(crate) mod nzchar;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "nzchar", None)
    }

    #[test]
    fn test_lint_nzchar() {
        assert_snapshot!(
            snapshot_lint("x == ''"),
            @r#"
        warning: nzchar
         --> <test>:1:1
          |
        1 | x == ''
          | ------- `x == ""` is inefficient.
          |
          = help: Use `!nzchar(x)` instead.
        Found 1 error.
        "#
        );

        assert_snapshot!(
            snapshot_lint("x != ''"),
            @r#"
        warning: nzchar
         --> <test>:1:1
          |
        1 | x != ''
          | ------- `x == ""` is inefficient.
          |
          = help: Use `!nzchar(x)` instead.
        Found 1 error.
        "#
        );

        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text(
                vec![
                    "x == ''",
                    "x != ''",
                    "foo(x(y)) == ''",
                    "'' == x",
                    "which(c(a, b, c) == '')"
                ],
                "nzchar",
            )
        );
    }

    #[test]
    fn test_no_lint_nzchar() {
        // `x %in% NaN` returns missings, but `NaN %in% x` returns TRUE/FALSE.
        expect_no_lint("'' %in% x", "nzchar", None);

        expect_no_lint("x %in% ''", "nzchar", None);

        expect_no_lint("x + ''", "nzchar", None);
    }

    #[test]
    fn test_nzchar_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_unsafe_fixed_text(
                vec![
                    "# leading comment\nx == ''",
                    "x # comment\n== ''",
                    "x == '' # trailing comment",
                ],
                "nzchar"
            )
        );
    }
}
