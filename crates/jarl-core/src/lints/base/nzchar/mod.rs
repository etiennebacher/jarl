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
          | ------- `x != ""` is inefficient.
          |
          = help: Use `nzchar(x)` instead.
        Found 1 error.
        "#
        );

        assert_snapshot!(
            snapshot_lint(r#"x == r"()""#),
            @r#"
        warning: nzchar
         --> <test>:1:1
          |
        1 | x == r"()"
          | ---------- `x == ""` is inefficient.
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
                    r#"x == r"()""#,
                    "foo(x(y)) == ''",
                    "'' == x",
                    "which(c(a, b, c) == '')"
                ],
                "nzchar",
            )
        );
    }

    #[test]
    fn test_lint_nchar_zero_comparisons() {
        assert_snapshot!(
            "nchar_zero_comparisons",
            snapshot_lint(concat!(
                "nchar(x) > 0\n",
                "nchar(x) != 0L\n",
                "nchar(x) <= 0.0\n",
                "nchar(x) == 0\n",
                "nchar(x) >= 0\n",
                "nchar(x) < 0\n",
                "0 < nchar(x)\n",
                "0 != nchar(x)\n",
                "0 >= nchar(x)\n",
                "0 == nchar(x)\n",
                "0 <= nchar(x)\n",
                "0 > nchar(x)\n",
                "nchar(x = x) > 0",
            ),)
        );

        assert_snapshot!(
            "nchar_zero_fix_output",
            get_unsafe_fixed_text(
                vec![
                    "nchar(x) > 0",
                    "nchar(x) != 0L",
                    "nchar(x) <= 0.0",
                    "nchar(x) == 0",
                    "nchar(x) >= 0",
                    "nchar(x) < 0",
                    "0 < nchar(x)",
                    "0 != nchar(x)",
                    "0 >= nchar(x)",
                    "0 <= nchar(x)",
                    "0 == nchar(x)",
                    "0 > nchar(x)",
                    "nchar(x = x) > 0",
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

        expect_no_lint(r#"x == "'"#, "nzchar", None);

        expect_no_lint(r#"x != "'"#, "nzchar", None);

        expect_no_lint(r#"x == "''"#, "nzchar", None);

        expect_no_lint(r#"x != "''"#, "nzchar", None);

        expect_no_lint("nchar(x) == 1", "nzchar", None);

        expect_no_lint("nchar(x, type = 'width') == 0", "nzchar", None);

        expect_no_lint("nchar(x, allowNA = TRUE) == 0", "nzchar", None);

        expect_no_lint("nchar(type = 'chars') == 0", "nzchar", None);

        expect_no_lint("nchar() == 0", "nzchar", None);
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
                    "nchar(x) # comment\n> 0",
                ],
                "nzchar"
            )
        );
    }
}
