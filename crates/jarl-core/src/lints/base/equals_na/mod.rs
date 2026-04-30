pub(crate) mod equals_na;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "equals_na", None)
    }

    #[test]
    fn test_lint_equals_na() {
        assert_snapshot!(
            snapshot_lint("x == NA"),
            @"
        warning: equals_na
         --> <test>:1:1
          |
        1 | x == NA
          | ------- Comparing to NA with `==`, `!=`, `%in%` or `%notin%` is problematic.
          |
          = help: Use `is.na()` or `!is.na()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x == NA_integer_"),
            @"
        warning: equals_na
         --> <test>:1:1
          |
        1 | x == NA_integer_
          | ---------------- Comparing to NA with `==`, `!=`, `%in%` or `%notin%` is problematic.
          |
          = help: Use `is.na()` or `!is.na()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x == NA_real_"),
            @"
        warning: equals_na
         --> <test>:1:1
          |
        1 | x == NA_real_
          | ------------- Comparing to NA with `==`, `!=`, `%in%` or `%notin%` is problematic.
          |
          = help: Use `is.na()` or `!is.na()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x == NA_logical_"),
            @"
      warning: equals_na
       --> <test>:1:1
        |
      1 | x == NA_logical_
        | ---------------- Comparing to NA with `==`, `!=`, `%in%` or `%notin%` is problematic.
        |
        = help: Use `is.na()` or `!is.na()` instead.
      Found 1 error.
      "
        );
        assert_snapshot!(
            snapshot_lint("x == NA_character_"),
            @"
        warning: equals_na
         --> <test>:1:1
          |
        1 | x == NA_character_
          | ------------------ Comparing to NA with `==`, `!=`, `%in%` or `%notin%` is problematic.
          |
          = help: Use `is.na()` or `!is.na()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x == NA_complex_"),
            @"
        warning: equals_na
         --> <test>:1:1
          |
        1 | x == NA_complex_
          | ---------------- Comparing to NA with `==`, `!=`, `%in%` or `%notin%` is problematic.
          |
          = help: Use `is.na()` or `!is.na()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x != NA"),
            @"
        warning: equals_na
         --> <test>:1:1
          |
        1 | x != NA
          | ------- Comparing to NA with `==`, `!=`, `%in%` or `%notin%` is problematic.
          |
          = help: Use `is.na()` or `!is.na()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x %in% NA"),
            @"
        warning: equals_na
         --> <test>:1:1
          |
        1 | x %in% NA
          | --------- Comparing to NA with `==`, `!=`, `%in%` or `%notin%` is problematic.
          |
          = help: Use `is.na()` or `!is.na()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x %notin% NA"),
            @"
        warning: equals_na
         --> <test>:1:1
          |
        1 | x %notin% NA
          | ------------ Comparing to NA with `==`, `!=`, `%in%` or `%notin%` is problematic.
          |
          = help: Use `is.na()` or `!is.na()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("foo(x(y)) == NA"),
            @"
        warning: equals_na
         --> <test>:1:1
          |
        1 | foo(x(y)) == NA
          | --------------- Comparing to NA with `==`, `!=`, `%in%` or `%notin%` is problematic.
          |
          = help: Use `is.na()` or `!is.na()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("NA == x"),
            @"
        warning: equals_na
         --> <test>:1:1
          |
        1 | NA == x
          | ------- Comparing to NA with `==`, `!=`, `%in%` or `%notin%` is problematic.
          |
          = help: Use `is.na()` or `!is.na()` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "x == NA",
                    "x == NA_integer_",
                    "x == NA_real_",
                    "x == NA_logical_",
                    "x == NA_character_",
                    "x == NA_complex_",
                    "x != NA",
                    "x %in% NA",
                    "x %notin% NA",
                    "x %notin% NA_character_",
                    "foo(x(y)) == NA",
                    "NA == x",
                ],
                "equals_na",
                None,
            )
        );
    }

    #[test]
    fn test_no_lint_equals_na() {
        // `NA %in% x` is equivalent to `anyNA(x)`, not `is.na(x)`
        expect_no_lint("NA %in% x", "equals_na", None);
        expect_no_lint("NA %notin% x", "equals_na", None);

        expect_no_lint("x + NA", "equals_na", None);
        expect_no_lint("x == \"NA\"", "equals_na", None);
        expect_no_lint("x == 'NA'", "equals_na", None);
        expect_no_lint("x <- NA", "equals_na", None);
        expect_no_lint("x <- NaN", "equals_na", None);
        expect_no_lint("x <- NA_real_", "equals_na", None);
        expect_no_lint("is.na(x)", "equals_na", None);
        expect_no_lint("is.nan(x)", "equals_na", None);
        expect_no_lint("x[!is.na(x)]", "equals_na", None);
        expect_no_lint("# x == NA", "equals_na", None);
        expect_no_lint("'x == NA'", "equals_na", None);
        expect_no_lint("x == f(NA)", "equals_na", None);
    }

    #[test]
    fn test_equals_na_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nx == NA",
                    "x # comment\n== NA",
                    "x %notin%\n# comment\nNA",
                    "# comment\nx == NA",
                    "x == NA # trailing comment",
                ],
                "equals_na",
                None
            )
        );
    }
}
