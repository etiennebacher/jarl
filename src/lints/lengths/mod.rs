pub(crate) mod lengths;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_lengths() {
        use insta::assert_snapshot;
        let expected_message = "Use `lengths()` to find the length";

        assert!(expect_lint(
            "sapply(x, length)",
            expected_message,
            "lengths"
        ));
        assert!(expect_lint(
            "sapply(x, FUN = length)",
            expected_message,
            "lengths"
        ));
        // TODO: the fix in this case is broken
        assert!(expect_lint(
            "sapply(FUN = length, x)",
            expected_message,
            "lengths"
        ));
        assert!(expect_lint(
            "vapply(x, length, integer(1))",
            expected_message,
            "lengths"
        ));

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
                "lengths"
            )
        );
    }

    #[test]
    fn test_no_lint_lengths() {
        assert!(no_lint("length(x)", "lengths"));
        assert!(no_lint("function(x) length(x) + 1L", "lengths"));
        assert!(no_lint("vapply(x, fun, integer(length(y)))", "lengths"));
        assert!(no_lint("sapply(x, sqrt, simplify = length(x))", "lengths"));
        assert!(no_lint("lapply(x, length)", "lengths"));
        assert!(no_lint("map(x, length)", "lengths"));
    }
}
