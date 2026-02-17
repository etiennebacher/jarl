pub(crate) mod length_test;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "length_test", None)
    }

    #[test]
    fn test_lint_length_test() {
        assert_snapshot!(
            snapshot_lint("length(x != 0)"),
            @r"
        warning: length_test
         --> <test>:1:1
          |
        1 | length(x != 0)
          | -------------- Checking the length of a logical vector is likely a mistake
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("length(x >= 0)"),
            @r"
        warning: length_test
         --> <test>:1:1
          |
        1 | length(x >= 0)
          | -------------- Checking the length of a logical vector is likely a mistake
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("length(x <= 0)"),
            @r"
        warning: length_test
         --> <test>:1:1
          |
        1 | length(x <= 0)
          | -------------- Checking the length of a logical vector is likely a mistake
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("length(x > 0)"),
            @r"
        warning: length_test
         --> <test>:1:1
          |
        1 | length(x > 0)
          | ------------- Checking the length of a logical vector is likely a mistake
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("length(x < 0)"),
            @r"
        warning: length_test
         --> <test>:1:1
          |
        1 | length(x < 0)
          | ------------- Checking the length of a logical vector is likely a mistake
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("length(x < 0)"),
            @r"
        warning: length_test
         --> <test>:1:1
          |
        1 | length(x < 0)
          | ------------- Checking the length of a logical vector is likely a mistake
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("length(x + y == 2)"),
            @r"
        warning: length_test
         --> <test>:1:1
          |
        1 | length(x + y == 2)
          | ------------------ Checking the length of a logical vector is likely a mistake
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "length(x != 0)",
                    "length(x >= 0)",
                    "length(x <= 0)",
                    "length(x > 0)",
                    "length(x < 0)",
                    "length(x < 0)",
                    "length(x + y == 2)"
                ],
                "length_test",
                None
            )
        );
    }

    #[test]
    fn test_no_lint_length_test() {
        expect_no_lint("length(x) > 0", "length_test", None);
        expect_no_lint("length(DF[key == val, cols])", "length_test", None);
    }

    #[test]
    fn test_length_test_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nlength(x != 0)",
                    "length(\n  # comment\n  x != 0\n)",
                    "length(x\n    # comment\n    >= 0)",
                    "length(x > 0) # trailing comment",
                ],
                "length_test",
                None
            )
        );
    }
}
