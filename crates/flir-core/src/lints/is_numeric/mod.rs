pub(crate) mod is_numeric;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

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
        use insta::assert_snapshot;

        let expected_message = "Use `is.numeric(x)` instead of";
        expect_lint(
            "is.numeric(x) || is.integer(x)",
            expected_message,
            "is_numeric",
            None,
        );

        // order doesn't matter
        expect_lint(
            "is.integer(x) || is.numeric(x)",
            expected_message,
            "is_numeric",
            None,
        );

        // identical expressions match too
        expect_lint(
            "is.integer(DT$x) || is.numeric(DT$x)",
            expected_message,
            "is_numeric",
            None,
        );

        // line breaks don't matter
        expect_lint(
            "
            if (
              is.integer(x)
              || is.numeric(x)
            ) TRUE
          ",
            expected_message,
            "is_numeric",
            None,
        );

        // caught when nesting
        expect_lint(
            "all(y > 5) && (is.integer(x) || is.numeric(x))",
            expected_message,
            "is_numeric",
            None,
        );

        // implicit nesting
        expect_lint(
            "is.integer(x) || is.numeric(x) || is.logical(x)",
            expected_message,
            "is_numeric",
            None,
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
}
