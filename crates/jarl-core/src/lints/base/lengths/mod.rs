pub(crate) mod lengths;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "lengths", None)
    }

    #[test]
    fn test_lint_lengths() {
        assert_snapshot!(
            snapshot_lint("sapply(x, length)"),
            @r"
        warning: lengths
         --> <test>:1:1
          |
        1 | sapply(x, length)
          | ----------------- Using `length()` on each element of a list is inefficient.
          |
          = help: Use `lengths()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("sapply(x, FUN = length)"),
            @r"
        warning: lengths
         --> <test>:1:1
          |
        1 | sapply(x, FUN = length)
          | ----------------------- Using `length()` on each element of a list is inefficient.
          |
          = help: Use `lengths()` instead.
        Found 1 error.
        "
        );
        // TODO: the fix in this case is broken
        assert_snapshot!(
            snapshot_lint("sapply(FUN = length, x)"),
            @r"
        warning: lengths
         --> <test>:1:1
          |
        1 | sapply(FUN = length, x)
          | ----------------------- Using `length()` on each element of a list is inefficient.
          |
          = help: Use `lengths()` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("vapply(x, length, integer(1))"),
            @r"
        warning: lengths
         --> <test>:1:1
          |
        1 | vapply(x, length, integer(1))
          | ----------------------------- Using `length()` on each element of a list is inefficient.
          |
          = help: Use `lengths()` instead.
        Found 1 error.
        "
        );

        // TODO: block purrr's usage (argument name is now .f)

        // TODO: how can I support pipes?

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "sapply(x, length)",
                    "sapply(x, FUN = length)",
                    "vapply(mtcars, length, integer(1))",
                ],
                "lengths",
                None
            )
        );
    }

    #[test]
    fn test_no_lint_lengths() {
        expect_no_lint("length(x)", "lengths", None);
        expect_no_lint("function(x) length(x) + 1L", "lengths", None);
        expect_no_lint("vapply(x, fun, integer(length(y)))", "lengths", None);
        expect_no_lint("sapply(x, sqrt, simplify = length(x))", "lengths", None);
        expect_no_lint("lapply(x, length)", "lengths", None);
        expect_no_lint("map(x, length)", "lengths", None);
    }

    #[test]
    fn test_lengths_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nsapply(x, length)",
                    "sapply(\n  # comment\n  x, length\n)",
                    "sapply(x,\n    # comment\n    length)",
                    "sapply(x, length) # trailing comment",
                ],
                "lengths",
                None
            )
        );
    }
}
