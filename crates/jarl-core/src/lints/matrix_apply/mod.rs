pub(crate) mod matrix_apply;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

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
        use insta::assert_snapshot;

        let expected_message = "is inefficient";
        expect_lint("apply(x, 1, sum)", expected_message, "matrix_apply", None);
        expect_lint(
            "apply(x, MARGIN = 1, FUN = sum)",
            expected_message,
            "matrix_apply",
            None,
        );
        expect_lint("apply(x, 1L, sum)", expected_message, "matrix_apply", None);
        expect_lint("apply(x, 1, mean)", expected_message, "matrix_apply", None);
        expect_lint(
            "apply(x, MARGIN = 1, FUN = mean)",
            expected_message,
            "matrix_apply",
            None,
        );
        expect_lint("apply(x, 1L, mean)", expected_message, "matrix_apply", None);

        expect_lint(
            "apply(x, 1, sum, na.rm = TRUE)",
            expected_message,
            "matrix_apply",
            None,
        );
        expect_lint(
            "apply(x, 1, sum, na.rm = FALSE)",
            expected_message,
            "matrix_apply",
            None,
        );
        expect_lint(
            "apply(x, 1, sum, na.rm = foo)",
            expected_message,
            "matrix_apply",
            None,
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
        use insta::assert_snapshot;
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
