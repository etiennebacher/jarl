pub(crate) mod redundant_ifelse;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_redundant_ifelse() {
        expect_no_lint("class(x) <- 'character'", "redundant_ifelse", None);
        expect_no_lint(
            "identical(class(x), c('glue', 'character'))",
            "redundant_ifelse",
            None,
        );
        expect_no_lint("all(sup %in% class(model))", "redundant_ifelse", None);

        // We cannot infer the use that will be made of this output, so we can't
        // report it:
        expect_no_lint(
            "is_regression <- class(x) == 'lm'",
            "redundant_ifelse",
            None,
        );
        expect_no_lint(
            "is_regression <- 'lm' == class(x)",
            "redundant_ifelse",
            None,
        );
        expect_no_lint(
            "is_regression <- \"lm\" == class(x)",
            "redundant_ifelse",
            None,
        );

        expect_no_lint("identical(foo(x), 'a')", "redundant_ifelse", None);
        expect_no_lint("identical(foo(x), c('a', 'b'))", "redundant_ifelse", None);
    }

    #[test]
    fn test_lint_redundant_ifelse() {
        use insta::assert_snapshot;

        let expected_message = "Comparing `class(x)` with";

        expect_lint(
            "if (class(x) == 'character') 1",
            expected_message,
            "redundant_ifelse",
            None,
        );
        expect_lint(
            "if (base::class(x) == 'character') 1",
            expected_message,
            "redundant_ifelse",
            None,
        );
        expect_lint(
            "if ('character' %in% class(x)) 1",
            expected_message,
            "redundant_ifelse",
            None,
        );
        expect_lint(
            "if (class(x) %in% 'character') 1",
            expected_message,
            "redundant_ifelse",
            None,
        );
        expect_lint(
            "if (class(x) != 'character') 1",
            expected_message,
            "redundant_ifelse",
            None,
        );
        expect_lint(
            "while (class(x) != 'character') 1",
            expected_message,
            "redundant_ifelse",
            None,
        );
        expect_lint(
            "x[if (class(x) == 'foo') 1 else 2]",
            expected_message,
            "redundant_ifelse",
            None,
        );

        // No fixes because we can't infer if it is correct or not.
        assert_snapshot!(
            "no_fix_output",
            get_fixed_text(
                vec!["is_regression <- class(x) == 'lm'",],
                "redundant_ifelse",
                None
            )
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "is_regression <- class(x) == 'lm'",
                    "if (class(x) == 'character') 1",
                    "is_regression <- 'lm' == class(x)",
                    "is_regression <- \"lm\" == class(x)",
                    "if ('character' %in% class(x)) 1",
                    "if (class(x) %in% 'character') 1",
                    "if (class(x) != 'character') 1",
                    "while (class(x) != 'character') 1",
                    "x[if (class(x) == 'foo') 1 else 2]",
                    "if (class(foo(bar(y) + 1)) == 'abc') 1",
                    "if (my_fun(class(x) != 'character')) 1",
                ],
                "redundant_ifelse",
                None
            )
        );
    }

    #[test]
    fn test_lint_identical_class() {
        use insta::assert_snapshot;

        let expected_message = "Using `identical(class(x), 'a')`";

        // Test identical() - these are always linted regardless of context
        expect_lint(
            "is_regression <- identical(class(x), 'lm')",
            expected_message,
            "redundant_ifelse",
            None,
        );
        expect_lint(
            "is_regression <- identical('lm', class(x))",
            expected_message,
            "redundant_ifelse",
            None,
        );
        expect_lint(
            "if (identical(class(x), 'character')) 1",
            expected_message,
            "redundant_ifelse",
            None,
        );
        expect_lint(
            "if (identical('character', class(x))) 1",
            expected_message,
            "redundant_ifelse",
            None,
        );
        expect_lint(
            "while (identical(class(x), 'foo')) 1",
            expected_message,
            "redundant_ifelse",
            None,
        );

        assert_snapshot!(
            "identical_class",
            get_fixed_text(
                vec![
                    "if (identical(class(x), 'character')) 1",
                    "if (identical('character', class(x))) 1",
                    "while (identical(class(x), 'foo')) 1",
                    "is_regression <- identical(class(x), 'lm')",
                    "is_regression <- identical('lm', class(x))",
                ],
                "redundant_ifelse",
                None
            )
        );
    }

    #[test]
    fn test_redundant_ifelse_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nif (class(x) == 'foo') 1",
                    "if(\n  class(\n  # comment\nx) == 'foo'\n) 1",
                    "if (class(x) == 'foo') 1 # trailing comment",
                ],
                "redundant_ifelse",
                None
            )
        );
    }
}
