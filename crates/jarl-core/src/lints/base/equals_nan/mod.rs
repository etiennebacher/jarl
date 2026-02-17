pub(crate) mod equals_nan;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "equals_nan", None)
    }

    #[test]
    fn test_lint_equals_nan() {
        assert_snapshot!(
            snapshot_lint("x == NaN"),
            @r"
        warning: equals_nan
         --> <test>:1:1
          |
        1 | x == NaN
          | -------- Comparing to NaN with `==`, `!=` or `%in%` is problematic.
          |
          = help: Use `is.nan()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x != NaN"),
            @r"
        warning: equals_nan
         --> <test>:1:1
          |
        1 | x != NaN
          | -------- Comparing to NaN with `==`, `!=` or `%in%` is problematic.
          |
          = help: Use `is.nan()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x %in% NaN"),
            @r"
        warning: equals_nan
         --> <test>:1:1
          |
        1 | x %in% NaN
          | ---------- Comparing to NaN with `==`, `!=` or `%in%` is problematic.
          |
          = help: Use `is.nan()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("foo(x(y)) == NaN"),
            @r"
        warning: equals_nan
         --> <test>:1:1
          |
        1 | foo(x(y)) == NaN
          | ---------------- Comparing to NaN with `==`, `!=` or `%in%` is problematic.
          |
          = help: Use `is.nan()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("NaN == x"),
            @r"
        warning: equals_nan
         --> <test>:1:1
          |
        1 | NaN == x
          | -------- Comparing to NaN with `==`, `!=` or `%in%` is problematic.
          |
          = help: Use `is.nan()` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "x == NaN",
                    "x != NaN",
                    "x %in% NaN",
                    "foo(x(y)) == NaN",
                    "NaN == x",
                ],
                "equals_nan",
                None,
            )
        );
    }

    #[test]
    fn test_no_lint_equals_nan() {
        // `x %in% NaN` returns missings, but `NaN %in% x` returns TRUE/FALSE.
        expect_no_lint("NaN %in% x", "equals_nan", None);

        expect_no_lint("x + NaN", "equals_nan", None);
        expect_no_lint("x == \"NaN\"", "equals_nan", None);
        expect_no_lint("x == 'NaN'", "equals_nan", None);
        expect_no_lint("x <- NaN", "equals_nan", None);
        expect_no_lint("is.nan(x)", "equals_nan", None);
        expect_no_lint("# x == NaN", "equals_nan", None);
        expect_no_lint("'x == NaN'", "equals_nan", None);
        expect_no_lint("x == f(NaN)", "equals_nan", None);
    }

    #[test]
    fn test_equals_nan_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nx == NaN",
                    "x # comment\n== NaN",
                    "x == NaN # trailing comment",
                ],
                "equals_nan",
                None
            )
        );
    }
}
