pub(crate) mod is_numeric;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "is_numeric", None)
    }

    #[test]
    fn test_no_lint_is_numeric() {
        expect_no_lint("is.numeric(x) || is.integer(y)", "is_numeric", None);
        expect_no_lint("is.numeric(x) || is.integer(foo(x))", "is_numeric", None);
        expect_no_lint("is.numeric(x) || is.integer(x[[1]])", "is_numeric", None);
        expect_no_lint("class(x) %in% 1:10", "is_numeric", None);
        expect_no_lint("class(x) %in% 'numeric'", "is_numeric", None);
        expect_no_lint(
            "class(x) %in% c('numeric', 'integer', 'factor')",
            "is_numeric",
            None,
        );
        expect_no_lint(
            "class(x) %in% c('numeric', 'integer', y)",
            "is_numeric",
            None,
        );
    }

    #[test]
    fn test_lint_is_numeric() {
        assert_snapshot!(
            snapshot_lint("is.numeric(x) || is.integer(x)"),
            @r"
        warning: is_numeric
         --> <test>:1:1
          |
        1 | is.numeric(x) || is.integer(x)
          | ------------------------------ `is.numeric(x) || is.integer(x)` is redundant.
          |
          = help: Use `is.numeric(x)` instead. Use `is.double(x)` to test for objects stored as 64-bit floating point
        Found 1 error.
        "
        );

        // order doesn't matter
        assert_snapshot!(
            snapshot_lint("is.integer(x) || is.numeric(x)"),
            @r"
        warning: is_numeric
         --> <test>:1:1
          |
        1 | is.integer(x) || is.numeric(x)
          | ------------------------------ `is.numeric(x) || is.integer(x)` is redundant.
          |
          = help: Use `is.numeric(x)` instead. Use `is.double(x)` to test for objects stored as 64-bit floating point
        Found 1 error.
        "
        );

        // identical expressions match too
        assert_snapshot!(
            snapshot_lint("is.integer(DT$x) || is.numeric(DT$x)"),
            @r"
        warning: is_numeric
         --> <test>:1:1
          |
        1 | is.integer(DT$x) || is.numeric(DT$x)
          | ------------------------------------ `is.numeric(x) || is.integer(x)` is redundant.
          |
          = help: Use `is.numeric(x)` instead. Use `is.double(x)` to test for objects stored as 64-bit floating point
        Found 1 error.
        "
        );

        // line breaks don't matter
        assert_snapshot!(snapshot_lint("
            if (
              is.integer(x)
              || is.numeric(x)
            ) TRUE
          "), @r"
        warning: is_numeric
         --> <test>:3:15
          |
        3 | /               is.integer(x)
        4 | |               || is.numeric(x)
          | |______________________________- `is.numeric(x) || is.integer(x)` is redundant.
          |
          = help: Use `is.numeric(x)` instead. Use `is.double(x)` to test for objects stored as 64-bit floating point
        Found 1 error.
        "
        );

        // caught when nesting
        assert_snapshot!(
            snapshot_lint("all(y > 5) && (is.integer(x) || is.numeric(x))"),
            @r"
        warning: is_numeric
         --> <test>:1:16
          |
        1 | all(y > 5) && (is.integer(x) || is.numeric(x))
          |                ------------------------------ `is.numeric(x) || is.integer(x)` is redundant.
          |
          = help: Use `is.numeric(x)` instead. Use `is.double(x)` to test for objects stored as 64-bit floating point
        Found 1 error.
        "
        );

        // implicit nesting
        assert_snapshot!(
            snapshot_lint("is.integer(x) || is.numeric(x) || is.logical(x)"),
            @r"
        warning: is_numeric
         --> <test>:1:1
          |
        1 | is.integer(x) || is.numeric(x) || is.logical(x)
          | ------------------------------ `is.numeric(x) || is.integer(x)` is redundant.
          |
          = help: Use `is.numeric(x)` instead. Use `is.double(x)` to test for objects stored as 64-bit floating point
        Found 1 error.
        "
        );
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "is.numeric(x) || is.integer(x)",
                    // order doesn't matter
                    "is.integer(x) || is.numeric(x)",
                    // identical expressions match too
                    "is.integer(DT$x) || is.numeric(DT$x)",
                    // line breaks don't matter
                    "if (
  is.integer(x)
  || is.numeric(x)
) TRUE",
                    // caught when nesting
                    "all(y > 5) && (is.integer(x) || is.numeric(x))",
                    // implicit nesting
                    "is.integer(x) || is.numeric(x) || is.logical(x)",
                ],
                "is_numeric",
                None
            )
        )
    }

    #[test]
    fn test_is_numeric_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nis.numeric(x) || is.integer(x)",
                    "is.numeric(\n  # comment\n  x\n) || is.integer(x)",
                    "is.integer(x) ||\n    # comment\n    is.numeric(x)",
                    "is.numeric(x) || is.integer(x) # trailing comment",
                ],
                "is_numeric",
                None
            )
        );
    }
}
