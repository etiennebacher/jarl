pub(crate) mod equals_null;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "equals_null", None)
    }

    #[test]
    fn test_lint_equals_null() {
        assert_snapshot!(
            snapshot_lint("x == NULL"),
            @r"
        warning: equals_null
         --> <test>:1:1
          |
        1 | x == NULL
          | --------- Comparing to NULL with `==`, `!=` or `%in%` is problematic.
          |
          = help: Use `is.null()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x != NULL"),
            @r"
        warning: equals_null
         --> <test>:1:1
          |
        1 | x != NULL
          | --------- Comparing to NULL with `==`, `!=` or `%in%` is problematic.
          |
          = help: Use `is.null()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x %in% NULL"),
            @r"
        warning: equals_null
         --> <test>:1:1
          |
        1 | x %in% NULL
          | ----------- Comparing to NULL with `==`, `!=` or `%in%` is problematic.
          |
          = help: Use `is.null()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("foo(x(y)) == NULL"),
            @r"
        warning: equals_null
         --> <test>:1:1
          |
        1 | foo(x(y)) == NULL
          | ----------------- Comparing to NULL with `==`, `!=` or `%in%` is problematic.
          |
          = help: Use `is.null()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("NULL == x"),
            @r"
        warning: equals_null
         --> <test>:1:1
          |
        1 | NULL == x
          | --------- Comparing to NULL with `==`, `!=` or `%in%` is problematic.
          |
          = help: Use `is.null()` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "x == NULL",
                    "x != NULL",
                    "x %in% NULL",
                    "foo(x(y)) == NULL",
                    "NULL == x",
                ],
                "equals_null",
                None,
            )
        );
    }

    #[test]
    fn test_no_lint_equals_null() {
        expect_no_lint("x + NULL", "equals_null", None);
        expect_no_lint("x == \"NULL\"", "equals_null", None);
        expect_no_lint("x == 'NULL'", "equals_null", None);
        expect_no_lint("x <- NULL", "equals_null", None);
        expect_no_lint("# x == NULL", "equals_null", None);
        expect_no_lint("'x == NULL'", "equals_null", None);
        expect_no_lint("x == f(NULL)", "equals_null", None);
    }

    #[test]
    fn test_equals_null_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nx == NULL",
                    "x # comment\n== NULL",
                    "x == NULL # trailing comment",
                ],
                "equals_null",
                None
            )
        );
    }
}
