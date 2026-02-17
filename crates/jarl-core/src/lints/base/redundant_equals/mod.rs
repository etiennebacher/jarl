pub(crate) mod redundant_equals;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "redundant_equals", None)
    }

    #[test]
    fn test_lint_redundant_equals() {
        assert_snapshot!(
            snapshot_lint("a == TRUE"),
            @r"
        warning: redundant_equals
         --> <test>:1:1
          |
        1 | a == TRUE
          | --------- Using == on a logical vector is redundant.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("TRUE == a"),
            @r"
        warning: redundant_equals
         --> <test>:1:1
          |
        1 | TRUE == a
          | --------- Using == on a logical vector is redundant.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("a == FALSE"),
            @r"
        warning: redundant_equals
         --> <test>:1:1
          |
        1 | a == FALSE
          | ---------- Using == on a logical vector is redundant.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("FALSE == a"),
            @r"
        warning: redundant_equals
         --> <test>:1:1
          |
        1 | FALSE == a
          | ---------- Using == on a logical vector is redundant.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("a != TRUE"),
            @r"
        warning: redundant_equals
         --> <test>:1:1
          |
        1 | a != TRUE
          | --------- Using == on a logical vector is redundant.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("TRUE != a"),
            @r"
        warning: redundant_equals
         --> <test>:1:1
          |
        1 | TRUE != a
          | --------- Using == on a logical vector is redundant.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("a != FALSE"),
            @r"
        warning: redundant_equals
         --> <test>:1:1
          |
        1 | a != FALSE
          | ---------- Using == on a logical vector is redundant.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("FALSE != a"),
            @r"
        warning: redundant_equals
         --> <test>:1:1
          |
        1 | FALSE != a
          | ---------- Using == on a logical vector is redundant.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "a == TRUE",
                    "TRUE == a",
                    "a == FALSE",
                    "FALSE == a",
                    "a != TRUE",
                    "TRUE != a",
                    "a != FALSE",
                    "FALSE != a",
                    "foo(a(b = 1)) == TRUE"
                ],
                "redundant_equals",
                None
            )
        );
    }

    #[test]
    fn test_no_lint_redundant_equals() {
        expect_no_lint("x == 1", "redundant_equals", None);
        expect_no_lint("x == 'TRUE'", "redundant_equals", None);
        expect_no_lint("x == 'FALSE'", "redundant_equals", None);
        expect_no_lint("x > 1", "redundant_equals", None);
    }

    #[test]
    fn test_redundant_equals_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\na == TRUE",
                    "a # comment\n== TRUE",
                    "# comment\na == FALSE",
                    "a == TRUE # trailing comment",
                ],
                "redundant_equals",
                None
            )
        );
    }
}
