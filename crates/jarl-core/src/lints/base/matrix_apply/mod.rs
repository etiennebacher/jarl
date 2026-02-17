pub(crate) mod matrix_apply;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "matrix_apply", None)
    }

    #[test]
    fn test_no_lint_matrix_apply() {
        expect_no_lint("apply(x, 1, prod)", "matrix_apply", None);
        expect_no_lint(
            "apply(x, 1, function(i) sum(i[i > 0]))",
            "matrix_apply",
            None,
        );
        expect_no_lint("apply(x, 1, f, sum)", "matrix_apply", None);
        expect_no_lint("apply(x, 1, mean, trim = 0.2)", "matrix_apply", None);
        expect_no_lint("apply(x, seq(2, 4), sum)", "matrix_apply", None);
        expect_no_lint("apply(x, c(2, 4), sum)", "matrix_apply", None);
        expect_no_lint("apply(x, m, sum)", "matrix_apply", None);
        expect_no_lint("apply(x, 1 + 2:4, sum)", "matrix_apply", None);

        // Do not panic (no arg value for `X`)
        expect_no_lint("apply(X=, 1, sum)", "matrix_apply", None);
    }

    #[test]
    fn test_lint_matrix_apply() {
        assert_snapshot!(
            snapshot_lint("apply(x, 1, sum)"),
            @r"
        warning: matrix_apply
         --> <test>:1:1
          |
        1 | apply(x, 1, sum)
          | ---------------- `apply(x, 1, sum)` is inefficient.
          |
          = help: Use `rowSums(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("base::apply(x, 1, sum)"),
            @r"
        warning: matrix_apply
         --> <test>:1:1
          |
        1 | base::apply(x, 1, sum)
          | ---------------------- `apply(x, 1, sum)` is inefficient.
          |
          = help: Use `rowSums(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("apply(x, MARGIN = 1, FUN = sum)"),
            @r"
        warning: matrix_apply
         --> <test>:1:1
          |
        1 | apply(x, MARGIN = 1, FUN = sum)
          | ------------------------------- `apply(x, 1, sum)` is inefficient.
          |
          = help: Use `rowSums(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("apply(x, 1L, sum)"),
            @r"
        warning: matrix_apply
         --> <test>:1:1
          |
        1 | apply(x, 1L, sum)
          | ----------------- `apply(x, 1, sum)` is inefficient.
          |
          = help: Use `rowSums(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("apply(x, 1, mean)"),
            @r"
        warning: matrix_apply
         --> <test>:1:1
          |
        1 | apply(x, 1, mean)
          | ----------------- `apply(x, 1, mean)` is inefficient.
          |
          = help: Use `rowMeans(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("apply(x, MARGIN = 1, FUN = mean)"),
            @r"
        warning: matrix_apply
         --> <test>:1:1
          |
        1 | apply(x, MARGIN = 1, FUN = mean)
          | -------------------------------- `apply(x, 1, mean)` is inefficient.
          |
          = help: Use `rowMeans(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("apply(x, 1L, mean)"),
            @r"
        warning: matrix_apply
         --> <test>:1:1
          |
        1 | apply(x, 1L, mean)
          | ------------------ `apply(x, 1, mean)` is inefficient.
          |
          = help: Use `rowMeans(x)` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(

            snapshot_lint("apply(x, 1, sum, na.rm = TRUE)"),

            @r"
        warning: matrix_apply
         --> <test>:1:1
          |
        1 | apply(x, 1, sum, na.rm = TRUE)
          | ------------------------------ `apply(x, 1, sum)` is inefficient.
          |
          = help: Use `rowSums(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("apply(x, 1, sum, na.rm = FALSE)"),
            @r"
        warning: matrix_apply
         --> <test>:1:1
          |
        1 | apply(x, 1, sum, na.rm = FALSE)
          | ------------------------------- `apply(x, 1, sum)` is inefficient.
          |
          = help: Use `rowSums(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("apply(x, 1, sum, na.rm = foo)"),
            @r"
        warning: matrix_apply
         --> <test>:1:1
          |
        1 | apply(x, 1, sum, na.rm = foo)
          | ----------------------------- `apply(x, 1, sum)` is inefficient.
          |
          = help: Use `rowSums(x)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "apply(x, 1, sum)",
                    "apply(x, 1L, sum)",
                    "apply(x, MARGIN = 1, FUN = sum)",
                    "apply(MARGIN = 1, FUN = sum, X = x)",
                    "apply(x, 1, mean)",
                    "apply(x, 1L, mean)",
                    "apply(x, MARGIN = 1, FUN = mean)",
                    "apply(x, 1, sum, na.rm = TRUE)",
                    "apply(x, 1, sum, na.rm = FALSE)",
                    "apply(x, 1, sum, na.rm = foo)",
                    "apply(x, 2, sum, na.rm = TRUE)",
                    "apply(x, 2, sum, na.rm = FALSE)",
                    "apply(x, 2, sum, na.rm = foo)",
                ],
                "matrix_apply",
                None
            )
        );
    }

    #[test]
    fn test_matrix_apply_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\napply(x, 1, sum)",
                    "apply(\n  # comment\n  x, 1, sum\n)",
                    "apply(x,\n    # comment\n    1, sum)",
                    "apply(x, 1, sum) # trailing comment",
                ],
                "matrix_apply",
                None
            )
        );
    }
}
